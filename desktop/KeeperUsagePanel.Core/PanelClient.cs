using System.Net;
using System.Text;
using System.Text.Json.Nodes;

namespace KeeperUsagePanel.Core;

/// <summary>Direct Keeper 1.14.8 HTTP client. No local or remote adapter service.</summary>
public sealed class PanelClient : IDisposable
{
    private readonly HttpClient http;
    private readonly string password;
    private readonly SemaphoreSlim loginGate = new(1,1);
    private readonly Func<DateTimeOffset> clock;
    private readonly Dictionary<string,(DateTimeOffset at,JsonNode data)> cache=[];
    private readonly Dictionary<string,JsonNode> identities=[];
    private readonly IntervalCounter counter=new();
    private bool authenticated;
    public PanelClient(string endpoint,string password,HttpMessageHandler? handler=null,Func<DateTimeOffset>? clock=null)
    {
        var uri=new Uri(endpoint.TrimEnd('/')+"/");
        if(uri.Scheme!="https"&&uri.Scheme!="http")throw new ArgumentException("仅支持 HTTP/HTTPS 地址");
        if(!string.IsNullOrEmpty(uri.UserInfo)||!string.IsNullOrEmpty(uri.Query)||!string.IsNullOrEmpty(uri.Fragment))throw new ArgumentException("地址不能包含用户名、查询参数或片段");
        this.password=password;this.clock=clock??(()=>DateTimeOffset.UtcNow);
        http=new HttpClient(handler??new HttpClientHandler{AllowAutoRedirect=false,UseCookies=true,CookieContainer=new CookieContainer()}){BaseAddress=uri,Timeout=TimeSpan.FromSeconds(35)};
    }
    private async Task<JsonNode> Request(string path,CancellationToken ct,JsonNode? body=null,bool retry=true)
    {
        if(path!="auth/login"&&!authenticated)await Login(ct);
        using var request=new HttpRequestMessage(body==null?HttpMethod.Get:HttpMethod.Post,"api/v1/"+path);
        if(body!=null){request.Content=new StringContent(body.ToJsonString(),Encoding.UTF8,"application/json");request.Headers.Add("X-CPA-Usage-Keeper-Request","fetch");}
        using var response=await http.SendAsync(request,ct);
        if(response.StatusCode==HttpStatusCode.Unauthorized)
        {
            authenticated=false;
            if(path!="auth/login"&&retry){await Login(ct);return await Request(path,ct,body,false);}
            throw new PanelException("需登录","Keeper 登录密码无效或会话失效。");
        }
        if(!response.IsSuccessStatusCode)
        {
            string message=response.StatusCode==HttpStatusCode.BadRequest?"Keeper 不支持此日期范围或账户类型。":response.StatusCode==HttpStatusCode.NotFound?"接口不存在，请检查 Keeper 地址是否包含 /usage 以及版本是否兼容。":"Keeper 暂不可用，请稍后重试。";
            throw new PanelException("连接异常",message);
        }
        string text=await response.Content.ReadAsStringAsync(ct);
        if(string.IsNullOrWhiteSpace(text))return new JsonObject();
        try{return JsonNode.Parse(text)??new JsonObject();}catch(System.Text.Json.JsonException){throw new PanelException("地址错误","Keeper 未返回 JSON，请检查地址及 /usage 路径。");}
    }
    private async Task Login(CancellationToken ct)
    {
        await loginGate.WaitAsync(ct);
        try{if(authenticated)return;await Request("auth/login",ct,new JsonObject{["password"]=password},false);authenticated=true;}
        finally{loginGate.Release();}
    }
    private async Task<JsonNode> Cached(string path,int seconds,CancellationToken ct)
    {
        lock(cache){if(cache.TryGetValue(path,out var item)&&(clock()-item.at).TotalSeconds<seconds)return item.data;}
        var data=await Request(path,ct);
        lock(cache){if(cache.Count>=12)cache.Clear();cache[path]=(clock(),data);}return data;
    }
    public async Task<JsonNode> SampleAsync(CancellationToken cancellation=default)
    {
        using var timeout=CancellationTokenSource.CreateLinkedTokenSource(cancellation);timeout.CancelAfter(TimeSpan.FromSeconds(15));var ct=timeout.Token;
        var today=await Request("usage/activity?window=today",ct);
        if(J.S(today,"timezone")!="Asia/Shanghai")throw new PanelException("时区不符","当前版本要求 Keeper 使用 Asia/Shanghai，避免今日指标按其它时区计数。");
        if(!DateTimeOffset.TryParse(J.S(today,"window_start"),out var start)||J.At(today,"input_tokens")==null||J.At(today,"output_tokens")==null)throw new PanelException("数据异常","今日累计字段缺失，保留上一次采样基线。");
        DateOnly day=DateOnly.FromDateTime(start.ToOffset(TimeSpan.FromHours(8)).DateTime);
        var at=clock();var current=new UsageReading(day,J.L(today,"input_tokens"),J.L(today,"output_tokens"),J.L(today,"total_success")+J.L(today,"total_failure"),at);
        UsageTotals? closedDays=null;
        if(counter.Previous is {} previous&&day>previous.Day&&day.DayNumber-previous.Day.DayNumber<=364)
        {
            // At midnight/reconnection, finish the old date(s), then add the new
            // day's cumulative value. Never subtract unrelated daily totals.
            string path=$"usage/analysis?range=custom&unit=day&start={previous.Day:yyyy-MM-dd}&end={day.AddDays(-1):yyyy-MM-dd}";
            var historical=await Request(path,ct);closedDays=SumTokens(historical);
        }
        // Reuse Keeper's exact ten-minute-aligned five-hour credential health.
        long success=0,failure=0;string healthStart="",healthEnd="";int pages=1;
        for(int page=1;page<=pages;page++)
        {
            var data=await Cached($"usage/identities/page?page_size=100&page={page}&active_only=false",10,ct);
            pages=Math.Max(1,(int)J.N(data,"total_pages"));
            foreach(var account in J.Items(data,"identities"))
            {
                success+=J.L(account,"credential_health.total_success");failure+=J.L(account,"credential_health.total_failure");
                healthStart=J.S(account,"credential_health.window_start",healthStart);healthEnd=J.S(account,"credential_health.window_end",healthEnd);
            }
        }
        var delta=counter.Accept(current,closedDays); // All required reads succeeded.
        return new JsonObject{["sampled_at"]=at.ToString("O"),["timezone"]="Asia/Shanghai",["today_tokens"]=J.L(today,"total_tokens"),
            ["delta"]=new JsonObject{["input_tokens"]=delta.Input,["output_tokens"]=delta.Output,["requests"]=delta.Requests,["seconds"]=delta.Seconds,["baseline"]=delta.Baseline,["reset"]=delta.Reset},
            ["health"]=new JsonObject{["label"]=Health.Label(success,failure),["success"]=success,["failure"]=failure,["start"]=healthStart,["end"]=healthEnd}};
    }
    // View names are in-process dispatch only; ALL network calls use Keeper api/v1.
    public async Task<JsonNode> GetViewAsync(string view,CancellationToken ct=default)
    {
        var pieces=view.Split('?',2);string path=pieces[0];var q=ParseQuery(pieces.Length>1?pieces[1]:"");string query=KeeperQuery(q);
        switch(path)
        {
            case "v1/keys":return await Cached("usage/api-keys/options",60,ct);
            case "v1/summary":
                var overview=await Cached("usage/overview?"+query,8,ct);
                var analysis=await Cached("usage/analysis?"+query,15,ct);var sums=SumTokens(analysis);
                return new JsonObject{["overview"]=overview.DeepClone(),["activity"]=new JsonObject{["input_tokens"]=sums.Input,["output_tokens"]=sums.Output,["total_tokens"]=sums.Total,["cache_read_tokens"]=sums.CacheRead,["cache_creation_tokens"]=sums.CacheWrite,["reasoning_tokens"]=sums.Reasoning}};
            case "v1/analysis":return await Cached("usage/analysis?"+query,15,ct);
            case "v1/latency":return await Cached("usage/analysis/latency?"+query,30,ct);
            case "v1/accounts":
                var accounts=await Cached("usage/identities/page?auth_type=1&page_size=20&page="+q.GetValueOrDefault("page","1"),30,ct);
                foreach(var account in J.Items(accounts,"identities")){if(account!=null)identities[J.S(account,"id")]=account;}
                return accounts;
        }
        var segments=path.Split('/');
        if(segments.Length!=4||segments[0]!="v1"||segments[1]!="accounts"||!identities.TryGetValue(segments[2],out var identity))throw new PanelException("数据异常","请重新选择账户。");
        string authIndex=J.S(identity,"identity","");string escaped=Uri.EscapeDataString(authIndex);
        switch(segments[3])
        {
            case "quota":return await Request("quota/cache",ct,new JsonObject{["auth_indexes"]=new JsonArray(authIndex)});
            case "quota-history":
                if(!string.Equals(J.S(identity,"type"),"codex",StringComparison.OrdinalIgnoreCase))return new JsonObject{["supported"]=false,["cycles"]=new JsonArray()};
                return await Cached("quota/history/"+escaped+"?window_role="+q.GetValueOrDefault("window_role","primary"),30,ct);
            case "requests":return await Request("usage/events?"+query+"&auth_type=1&source="+escaped+"&page_size=50&cursor_mode=true"+CursorQuery(q),ct);
            case "errors":
                // Keeper has no error date/key filter. Filter a cursor page locally,
                // and report page count instead of an invented range-wide total.
                var errors=await Request("usage/identities/"+Uri.EscapeDataString(segments[2])+"/errors?page_size=50"+CursorQuery(q),ct);
                var (from,to)=RangeQuery.Bounds(q,clock());var events=new JsonArray();DateTimeOffset? oldest=null;
                foreach(var item in J.Items(errors,"events"))if(DateTimeOffset.TryParse(J.S(item,"timestamp"),out var t)){oldest=t;if(t>=from&&t<to)events.Add(item!.DeepClone());}
                bool more=J.B(errors,"has_more")&&(oldest==null||oldest>=from);
                return new JsonObject{["events"]=events,["total_count"]=events.Count,["has_more"]=more,["next_cursor"]=J.S(errors,"next_cursor",""),["scope_notice"]="日期在当前游标页内筛选；只显示本页条数，不代表日期范围总数。较早记录可翻页查看。错误不支持按 Key 归属。"};
        }
        throw new PanelException("数据异常","未知视图。");
    }
    private string KeeperQuery(Dictionary<string,string> values)
    {
        var q=new Dictionary<string,string>();string range=values.GetValueOrDefault("range","today");
        if(range=="month"){var today=clock().ToOffset(TimeSpan.FromHours(8));q["range"]="custom";q["unit"]="day";q["start"]=today.ToString("yyyy-MM-01");q["end"]=today.ToString("yyyy-MM-dd");}
        else{q["range"]=range;if(range=="custom"){q["unit"]="day";q["start"]=values.GetValueOrDefault("start","");q["end"]=values.GetValueOrDefault("end","");}}
        if(values.TryGetValue("api_key_id",out var key)&&key.Length>0)q["api_key_id"]=key;
        return string.Join("&",q.Select(pair=>Uri.EscapeDataString(pair.Key)+"="+Uri.EscapeDataString(pair.Value)));
    }
    private static string CursorQuery(Dictionary<string,string> q)=>q.TryGetValue("cursor",out var cursor)?"&cursor="+Uri.EscapeDataString(cursor):"";
    private static Dictionary<string,string> ParseQuery(string query)=>query.Split('&',StringSplitOptions.RemoveEmptyEntries).Select(p=>p.Split('=',2)).ToDictionary(p=>Uri.UnescapeDataString(p[0]),p=>p.Length>1?Uri.UnescapeDataString(p[1]):"");
    private static UsageTotals SumTokens(JsonNode data)
    {
        long input=0,output=0,requests=0,total=0,read=0,write=0,reasoning=0;
        foreach(var row in J.Items(data,"token_usage")){input+=J.L(row,"input_tokens");output+=J.L(row,"output_tokens");requests+=J.L(row,"requests");total+=J.L(row,"total_tokens");read+=J.L(row,"cache_read_tokens");write+=J.L(row,"cache_creation_tokens");reasoning+=J.L(row,"reasoning_tokens");}
        return new(input,output,requests,total,read,write,reasoning);
    }
    public void Dispose()=>http.Dispose();
}
