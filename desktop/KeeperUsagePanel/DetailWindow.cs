using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Threading;
using KeeperUsagePanel.Core;

namespace KeeperUsagePanel;

internal sealed record Choice(string Id,string Label) { public override string ToString()=>Label; }

internal sealed class DetailWindow:Window
{
    private readonly Func<PanelClient?> getClient;
    private readonly ComboBox range=Theme.Combo(106),keys=Theme.Combo(210);
    private readonly DatePicker start=new(){Width=150},end=new(){Width=150};
    private readonly StackPanel custom=new(){Orientation=Orientation.Horizontal,Margin=new Thickness(0,8,0,0)},body=new(),navigation=new(){Orientation=Orientation.Horizontal};
    private readonly TextBlock status=Theme.Text("等待采样",10,Theme.Muted),scope=Theme.Text("北京时间 · 全部 Key",10,Theme.Muted);
    private readonly DispatcherTimer refresh=new(){Interval=TimeSpan.FromSeconds(2)};
    private CancellationTokenSource? loading;
    private bool initializing;
    private int tab,accountTab,accountPage=1;
    private string accountId="",cursor="",role="primary",distribution="model_composition";
    private JsonNode? sample;
    private int pollInterval=2;
    private DateTime loadedAt=DateTime.MinValue;
    private static readonly string[] Tabs=["总览","成本","延迟","分布","账户"];
    public bool HasOpenPopup=>HasPopup(this);
    private static bool HasPopup(DependencyObject parent)
    {
        if(parent is ComboBox c&&c.IsDropDownOpen||parent is DatePicker d&&d.IsDropDownOpen)return true;
        for(int i=0;i<VisualTreeHelper.GetChildrenCount(parent);i++)if(HasPopup(VisualTreeHelper.GetChild(parent,i)))return true;return false;
    }
    public DetailWindow(Func<PanelClient?> client,Action settings)
    {
        getClient=client;Title="Keeper · Usage Panel";Width=548;Height=690;WindowStyle=WindowStyle.None;ResizeMode=ResizeMode.NoResize;Topmost=true;ShowActivated=false;ShowInTaskbar=false;Theme.Apply(this);
        var shell=new Border{BorderBrush=Theme.Line,BorderThickness=new Thickness(1),Padding=new Thickness(22,18,14,14)};Content=shell;
        var root=new DockPanel();shell.Child=root;
        var header=new StackPanel{Margin=new Thickness(0,0,8,0)};DockPanel.SetDock(header,Dock.Top);root.Children.Add(header);
        var title=new DockPanel();var setup=Theme.Button("设置",settings);DockPanel.SetDock(setup,Dock.Right);title.Children.Add(setup);title.Children.Add(Theme.Number("KEEPER / USAGE",21,Theme.Accent));header.Children.Add(title);
        status.Margin=new Thickness(0,4,0,15);header.Children.Add(status);
        var filters=new StackPanel{Orientation=Orientation.Horizontal};header.Children.Add(filters);
        range.ItemsSource=new[]{new Choice("today","今日"),new Choice("yesterday","昨日"),new Choice("7d","近 7 天"),new Choice("30d","近 30 天"),new Choice("month","本月"),new Choice("custom","自定义")};range.SelectedIndex=0;filters.Children.Add(range);
        keys.Items.Add(new Choice("","全部 Key"));keys.SelectedIndex=0;filters.Children.Add(keys);filters.Children.Add(Theme.Button("刷新",()=>{cursor="";_=ReloadAsync();}));
        start.SelectedDate=DateTime.UtcNow.AddHours(8).Date;end.SelectedDate=start.SelectedDate;custom.Children.Add(start);custom.Children.Add(Theme.Text(" 至 ",11));custom.Children.Add(end);custom.Children.Add(Theme.Button("应用",()=>{cursor="";_=ReloadAsync();}));custom.Visibility=Visibility.Collapsed;header.Children.Add(custom);
        scope.Margin=new Thickness(0,10,0,12);header.Children.Add(scope);header.Children.Add(navigation);BuildNavigation();
        var footer=Theme.Text("只读 · 不刷新上游额度 · 不改变 Keeper 数据",10,Theme.Muted);footer.Margin=new Thickness(0,10,0,0);DockPanel.SetDock(footer,Dock.Bottom);root.Children.Add(footer);
        root.Children.Add(new ScrollViewer{Content=body,VerticalScrollBarVisibility=ScrollBarVisibility.Auto,HorizontalScrollBarVisibility=ScrollBarVisibility.Disabled,Margin=new Thickness(0,10,0,0)});
        range.SelectionChanged+=(_,_)=>{custom.Visibility=(range.SelectedItem as Choice)?.Id=="custom"?Visibility.Visible:Visibility.Collapsed;if(!initializing&&IsVisible){cursor="";_=ReloadAsync();}};
        keys.SelectionChanged+=(_,_)=>{if(!initializing&&IsVisible){cursor="";_=ReloadAsync();}};
        SourceInitialized+=(_,_)=>Native.Configure(this,false);
        IsVisibleChanged+=(_,_)=>{if(!IsVisible){loading?.Cancel();refresh.Stop();}};
        refresh.Tick+=(_,_)=>{int seconds=tab==0?10:tab==4?60:30;if(IsVisible&&!HasOpenPopup&&(DateTime.UtcNow-loadedAt).TotalSeconds>=seconds)_=ReloadAsync();};
    }
    internal void Open()
    {
        initializing=true;range.SelectedIndex=0;keys.SelectedIndex=0;tab=0;accountTab=0;cursor="";BuildNavigation();initializing=false;
        Show();refresh.Start();_=LoadKeysThenReload();
    }
    private async Task LoadKeysThenReload()
    {
        loading?.Cancel();loading=new CancellationTokenSource();var ct=loading.Token;
        try{var client=getClient();if(client!=null){var data=await client.GetViewAsync("v1/keys",ct);if(ct.IsCancellationRequested||!IsVisible)return;initializing=true;keys.Items.Clear();keys.Items.Add(new Choice("","全部 Key"));foreach(var item in J.Items(data,"options"))keys.Items.Add(new Choice(J.S(item,"id",""),J.S(item,"label")));keys.SelectedIndex=0;initializing=false;}}
        catch(OperationCanceledException){return;}catch{initializing=false;}
        if(IsVisible)await ReloadAsync();
    }
    private void BuildNavigation()
    {
        navigation.Children.Clear();for(int i=0;i<Tabs.Length;i++){int selected=i;navigation.Children.Add(Theme.Button(Tabs[i],()=>{tab=selected;cursor="";BuildNavigation();_=ReloadAsync();},tab==i));}
    }
    internal void Stop(){refresh.Stop();loading?.Cancel();}
    internal void SetSample(JsonNode value,int interval){sample=value;pollInterval=interval;status.Text="● 已连接  ·  "+J.Time(J.S(value,"sampled_at"))+"  ·  北京时间";status.Foreground=Theme.Muted;}
    internal void SetConnectionState(string state){status.Text=state+" · 已显示的数据可能过期";status.Foreground=Theme.Amber;}
    private string Query()=>RangeQuery.Build((range.SelectedItem as Choice)?.Id??"today",(keys.SelectedItem as Choice)?.Id??"",start.SelectedDate,end.SelectedDate);
    private async Task ReloadAsync()
    {
        if(!IsVisible)return;loading?.Cancel();loading=new CancellationTokenSource();var ct=loading.Token;loadedAt=DateTime.UtcNow;
        body.Children.Clear();Theme.Note(body,"正在读取 Keeper…");
        try
        {
            var client=getClient();if(client==null){body.Children.Clear();Theme.Heading(body,"尚未连接");Theme.Note(body,"首次启动请在设置中填写 Keeper 地址和登录密码。");return;}
            string query=Query();scope.Text=$"北京时间 · {(range.SelectedItem as Choice)?.Label} · {(keys.SelectedItem as Choice)?.Label}";
            if(tab==4){await RenderAccounts(client,query,ct);return;}
            string endpoint=tab==0?"summary":tab==2?"latency":"analysis";
            var data=await client.GetViewAsync("v1/"+endpoint+"?"+query,ct);if(ct.IsCancellationRequested||!IsVisible)return;
            body.Children.Clear();switch(tab){case 0:RenderSummary(data);break;case 1:RenderCost(data);break;case 2:RenderLatency(data);break;case 3:RenderDistribution(data);break;}
        }
        catch(OperationCanceledException)when(ct.IsCancellationRequested){}
        catch(Exception ex){if(ct.IsCancellationRequested)return;body.Children.Clear();Theme.Heading(body,"暂时无法读取");Theme.Note(body,ex is PanelException?ex.Message:"连接超时或数据不可用，请检查设置后重试。");body.Children.Add(Theme.Button("重试",()=>_=ReloadAsync()));}
    }
    private void RenderSummary(JsonNode data)
    {
        var usage=J.At(data,"overview.usage");var sum=J.At(data,"overview.summary");var activity=data["activity"];
        Theme.Cards(body,("Token 总量",J.Count(J.N(usage,"total_tokens")),"输入与输出合计"),("Keeper 请求数",J.Count(J.N(usage,"total_requests")),"包含同步用量事件"),("缓存读取率",J.Percent(J.N(sum,"cache_read_tokens"),J.N(sum,"input_tokens")),"缓存读 / 输入（含缓存）"),("估算成本",J.Cost(sum,"total_cost"),"API 等价估算"));
        Theme.Heading(body,"用量组成");Theme.Table(body,["指标","结果"],new[]{new[]{"输入（含缓存）",J.Count(J.N(activity,"input_tokens"))},new[]{"输出（含推理）",J.Count(J.N(activity,"output_tokens"))},new[]{"缓存读取",J.Count(J.N(activity,"cache_read_tokens"))},new[]{"缓存写入",J.Count(J.N(activity,"cache_creation_tokens"))},new[]{"推理输出",J.Count(J.N(activity,"reasoning_tokens"))},new[]{"成功 / 失败",J.Count(J.N(usage,"success_count"))+" / "+J.Count(J.N(usage,"failure_count"))},new[]{"成功率",J.Percent(J.N(usage,"success_count"),J.N(usage,"total_requests"))}});
        Theme.Note(body,"缓存、推理属于子项，不再次计入总 Token。成功率沿用 Keeper 记录口径，不代表同步客户端完整的失败遥测。");
        if(sample!=null)
        {
            Theme.Heading(body,"全局实时 · 不受上方筛选影响");
            if(J.B(sample,"delta.baseline"))Theme.Note(body,"已建立采样基线，下一次成功采样显示新增用量。");
            else Theme.Note(body,$"相邻采样 {J.N(sample,"delta.seconds"):0.0} 秒：输入 +{J.Count(J.N(sample,"delta.input_tokens"))}，输出 +{J.Count(J.N(sample,"delta.output_tokens"))}。"+(J.N(sample,"delta.seconds")>pollInterval*2+1?"包含断线或暂停期间累计增长的用量。":""));
            Theme.Note(body,$"{J.S(sample,"health.label")} · Keeper 可见账户五小时窗口失败 {J.Count(J.N(sample,"health.failure"))} 次");
        }
    }
    private void RenderCost(JsonNode data)
    {
        var b=data["cost_breakdown"];
        Theme.Cards(body,("估算总成本",J.Cost(b,"total_cost_usd"),"Keeper 定价"),("普通输入",J.Cost(b,"uncached_input_cost_usd"),"不含缓存读 / 写"),("缓存读取",J.Cost(b,"cache_read_cost_usd"),null),("缓存写入",J.Cost(b,"cache_write_cost_usd"),null),("输出",J.Cost(b,"output_cost_usd"),"不重复加推理子项"));
        Theme.Heading(body,"模型效率");Theme.Table(body,["模型","请求","成本 / 请求","输出 / 请求","缓存率"],J.Items(data,"model_efficiency").Select(m=>new[]{J.S(m,"model"),J.Count(J.N(m,"requests")),J.Cost(m,"cost_per_request_usd"),J.N(m,"output_tokens_per_request").ToString("0.0"),J.Percent(J.N(m,"cache_read_tokens"),J.N(m,"input_tokens"))}));
        Theme.Note(body,"成本按 Keeper 价格配置估算；订阅同步用量不是实际扣费。价格缺失显示不可用，不补成 $0。");
    }
    private void RenderLatency(JsonNode data)
    {
        if(J.At(data,"supported")!=null&&!J.B(data,"supported")){Theme.Heading(body,"此范围不支持延迟统计");Theme.Note(body,"Keeper 延迟诊断限最近 30 天。请缩短日期范围。");return;}
        if(J.N(data,"total_points")==0){Theme.Heading(body,"暂无有效延迟样本");Theme.Note(body,"没有上报延迟的记录不视为 0 ms 请求。");return;}
        Theme.Cards(body,("首 Token · P95",J.Duration(J.N(data,"p95_ttft_ms")),"TTFT"),("请求耗时 · P95",J.Duration(J.N(data,"p95_latency_ms")),"端到端延迟"),("最慢首 Token",J.Duration(J.N(data,"max_ttft_ms")),null),("最长请求耗时",J.Duration(J.N(data,"max_latency_ms")),null));
        Theme.Note(body,"Keeper 统计样本："+J.Count(J.N(data,"total_points"))+"。"+(J.B(data,"sampled")?"上游图形点经过采样；这里直接采用其汇总值。":""));
        Theme.Note(body,"仅代表已上报且有效的延迟样本。Usage-Sync 导入的零延迟不表示请求瞬间完成。不平均时间桶的 P95，也不从散点反推全量分位数。");
    }
    private void RenderDistribution(JsonNode data)
    {
        var selector=Theme.Combo(180);var options=new[]{new Choice("model_composition","按模型"),new Choice("api_key_composition","按 Key"),new Choice("auth_files_composition","按认证账户"),new Choice("ai_provider_composition","按提供商")};selector.ItemsSource=options;selector.SelectedItem=options.First(x=>x.Id==distribution);body.Children.Add(selector);
        selector.SelectionChanged+=(_,_)=>{distribution=((Choice)selector.SelectedItem).Id;body.Children.Clear();RenderDistribution(data);};
        Theme.Heading(body,"当前范围的用量分布");Theme.Table(body,["名称","Token","占比","请求","估算成本"],J.Items(data,distribution).OrderByDescending(m=>J.N(m,"total_tokens")).Select(m=>new[]{J.S(m,"label"),J.Count(J.N(m,"total_tokens")),J.N(m,"percent").ToString("0.00")+"%",J.Count(J.N(m,"requests")),J.Cost(m)}));
        Theme.Note(body,"占比以当前范围 Token 总量为分母。模型、Key、账户与提供商是同一批用量的不同视角，不能相加。");
    }
    private async Task RenderAccounts(PanelClient client,string query,CancellationToken ct)
    {
        var data=await client.GetViewAsync("v1/accounts?page="+accountPage,ct);if(ct.IsCancellationRequested)return;
        var accounts=J.Items(data,"identities").Where(x=>x!=null).ToList();body.Children.Clear();
        Theme.Heading(body,"认证文件中的账户");
        if(accounts.Count==0){Theme.Note(body,"暂无认证账户。");return;}
        var selected=accounts.FirstOrDefault(x=>J.S(x,"id")==accountId)??accounts[0];accountId=J.S(selected,"id");
        var row=new StackPanel{Orientation=Orientation.Horizontal};var combo=Theme.Combo(300);combo.ItemsSource=accounts.Select(x=>new Choice(J.S(x,"id"),J.S(x,"displayName",J.S(x,"name")))).ToArray();combo.SelectedItem=combo.Items.Cast<Choice>().First(x=>x.Id==accountId);row.Children.Add(combo);body.Children.Add(row);
        if(accountPage>1)row.Children.Add(Theme.Button("上一页",()=>{accountPage--;accountId="";cursor="";_=ReloadAsync();}));
        if(accountPage<J.N(data,"total_pages"))row.Children.Add(Theme.Button("下一页",()=>{accountPage++;accountId="";cursor="";_=ReloadAsync();}));
        combo.SelectionChanged+=(_,_)=>{accountId=((Choice)combo.SelectedItem).Id;cursor="";_=ReloadAsync();};
        var tabs=new WrapPanel{Margin=new Thickness(0,12,0,0)};body.Children.Add(tabs);string[] names=["概览","额度历史","请求明细","错误事件"];
        for(int i=0;i<names.Length;i++){int next=i;tabs.Children.Add(Theme.Button(names[i],()=>{accountTab=next;cursor="";_=ReloadAsync();},accountTab==i));}
        switch(accountTab)
        {
            case 0:
                Theme.Note(body,"账户累计概览与当前配额，不受日期 / Key 筛选影响。");
                double success=J.N(selected,"success_count"),failure=J.N(selected,"failure_count");
                Theme.Cards(body,("累计请求",J.Count(J.N(selected,"total_requests")),null),("累计 Token",J.Count(J.N(selected,"total_tokens")),null),("累计成功率",J.Percent(success,success+failure),null),("累计缓存率",J.Percent(J.N(selected,"cache_read_tokens"),J.N(selected,"input_tokens")),null));
                Theme.Note(body,$"提供商：{J.S(selected,"provider")} · 类型：{J.S(selected,"type")} · {(J.B(selected,"disabled")?"已禁用":"已启用")}\n最近使用：{J.Time(J.S(selected,"last_used_at"))}");
                var health=selected?["credential_health"];if(health!=null)Theme.Note(body,$"最近 5 小时：成功 {J.Count(J.N(health,"total_success"))} / 失败 {J.Count(J.N(health,"total_failure"))}");
                var quota=await client.GetViewAsync($"v1/accounts/{accountId}/quota",ct);if(ct.IsCancellationRequested)return;RenderQuota(quota);break;
            case 1:
                Theme.Note(body,"账户共享配额 · 按真实周期统计 · 最近 30 天已有观测，不按 Key 拆分。");
                var roles=new StackPanel{Orientation=Orientation.Horizontal,Margin=new Thickness(0,8,0,8)};body.Children.Add(roles);roles.Children.Add(Theme.Button("主额度",()=>{role="primary";_=ReloadAsync();},role=="primary"));roles.Children.Add(Theme.Button("次额度",()=>{role="secondary";_=ReloadAsync();},role=="secondary"));
                var history=await client.GetViewAsync($"v1/accounts/{accountId}/quota-history?window_role={role}",ct);if(ct.IsCancellationRequested)return;RenderHistory(history);break;
            case 2:
                Theme.Note(body,"按上方日期、Key 和当前账户筛选。原始请求正文不读取。");
                var requests=await client.GetViewAsync($"v1/accounts/{accountId}/requests?{query}"+CursorQuery(),ct);if(ct.IsCancellationRequested)return;
                Theme.Table(body,["时间（北京）","模型","结果","输入","输出","缓存读","推理","Token 总数","估算成本","首 Token","耗时"],J.Items(requests,"events").Select(m=>new[]{J.Time(J.S(m,"timestamp")),J.S(m,"model"),J.B(m,"failed")?"失败":"成功",J.Count(J.N(m,"tokens.input_tokens")),J.Count(J.N(m,"tokens.output_tokens")),J.Count(J.N(m,"tokens.cache_read_tokens")),J.Count(J.N(m,"tokens.reasoning_tokens")),J.Count(J.N(m,"tokens.total_tokens")),J.Cost(m),J.Duration(J.N(m,"ttft_ms")),J.Duration(J.N(m,"latency_ms"))}));Pagination(requests);break;
            case 3:
                Theme.Note(body,"账户错误按日期筛选；不支持 Key 归属。错误事件数不能与失败请求数相加。");
                var errors=await client.GetViewAsync($"v1/accounts/{accountId}/errors?{query}"+CursorQuery(),ct);if(ct.IsCancellationRequested)return;
                Theme.Heading(body,"本页范围内错误事件："+J.Count(J.N(errors,"total_count")));
                Theme.Table(body,["时间（北京）","模型","HTTP","错误码","原因","可重试","账户重试时间","模型重试时间"],J.Items(errors,"events").Select(m=>new[]{J.Time(J.S(m,"timestamp")),J.S(m,"model"),J.S(m,"status_code"),J.S(m,"code"),J.S(m,"body_summary"),J.B(m,"retryable")?"是":"否",J.Time(J.S(m,"credential_retry_after")),J.Time(J.S(m,"model_retry_after"))}));Pagination(errors);Theme.Note(body,J.S(errors,"scope_notice"));break;
        }
    }
    private string CursorQuery()=>cursor.Length>0?"&cursor="+Uri.EscapeDataString(cursor):"";
    private void Pagination(JsonNode response)
    {
        var row=new StackPanel{Orientation=Orientation.Horizontal,Margin=new Thickness(0,12,0,0)};body.Children.Add(row);
        if(cursor.Length>0)row.Children.Add(Theme.Button("返回首屏",()=>{cursor="";_=ReloadAsync();}));
        if(J.B(response,"has_more")){string next=J.S(response,"next_cursor","");row.Children.Add(Theme.Button("下一页",()=>{cursor=next;_=ReloadAsync();},true));}
    }
    private void RenderQuota(JsonNode data)
    {
        Theme.Heading(body,"当前配额 · 读取已有缓存");bool any=false;
        foreach(var item in J.Items(data,"items"))
        {
            if(J.S(item,"status")=="failed"){Theme.Note(body,"Keeper 当前配额缓存不可用。");continue;}
            Theme.Note(body,"最近观测："+J.Time(J.S(item,"refreshed_at"))+" · 缓存到期："+J.Time(J.S(item,"expires_at")));
            var quota=item?["quota"];Theme.Note(body,"套餐："+J.S(quota,"subscription.plan"));
            var rows=J.Items(quota,"quota").ToList();any|=rows.Count>0;
            Theme.Table(body,["额度窗口","剩余 / 已用","重置时间","周期 Token","周期估算成本"],rows.Select(m=>new[]{J.S(m,"label",J.S(m,"key")),J.At(m,"remainingFraction")!=null?"剩余 "+(J.N(m,"remainingFraction")*100).ToString("0.0")+"%":J.At(m,"usedPercent")!=null?"已用 "+J.N(m,"usedPercent").ToString("0.0")+"%":J.S(m,"remaining"),J.Time(J.S(m,"resetAt")),J.At(m,"window_usage_tokens")!=null?J.Count(J.N(m,"window_usage_tokens")):"—",J.At(m,"window_usage_cost")!=null?"$"+J.N(m,"window_usage_cost").ToString("0.0000"):"—"}));
        }
        if(!any)Theme.Note(body,"暂无已缓存配额。此面板不会主动刷新上游额度。");
    }
    private void RenderHistory(JsonNode data)
    {
        if(J.At(data,"supported")!=null&&!J.B(data,"supported")){Theme.Note(body,"此账户类型暂不支持 Keeper 额度历史。");return;}
        var cycles=J.Items(data,"cycles").ToList();
        Theme.Table(body,["状态","周期开始","重置时间","初始剩余","最近剩余","请求","Token","估算成本"],cycles.Select(m=>new[]{J.S(m,"status")=="current"?"当前":"已结束",J.Time(J.S(m,"window_started_at")),J.Time(J.S(m,"reset_at")),J.S(m,"first_remaining_percent")+"%",J.S(m,"last_remaining_percent")+"%",J.Count(J.N(m,"usage.requests")),J.Count(J.N(m,"usage.total_tokens")),J.Cost(m,"usage.total_cost_usd","usage.cost_available")}));
        Theme.Heading(body,"额度变化效率");
        Theme.Table(body,["观察结束","额度变化","Token","每百分点 Token","每百分点成本"],cycles.SelectMany(c=>J.Items(c,"transitions")).Select(m=>new[]{J.Time(J.S(m,"interval_ended_at")),J.S(m,"from_remaining_percent")+"% → "+J.S(m,"to_remaining_percent")+"%",J.Count(J.N(m,"usage.total_tokens")),J.Count(J.N(m,"tokens_per_point")),J.Cost(m,"cost_per_point","cost_per_point_available")}));
        Theme.Note(body,"只显示真实观察到的周期和百分比变化；没有观测的日期不补零，不把百分点等同于 Token。");
    }
}
