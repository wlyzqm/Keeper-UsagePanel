using System;
using System.Text.Json.Nodes;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using KeeperUsagePanel.Core;

namespace KeeperUsagePanel;

internal sealed class WidgetWindow:Window
{
    private readonly TextBlock total=Theme.Number("—",29,Theme.Accent),input=Theme.Text("入 —",11),output=Theme.Text("出 —",11),health=Theme.Text("未连接",11,Theme.Muted);
    internal bool Dragging {get;private set;}
    internal bool MenuOpen=>ContextMenu?.IsOpen==true;
    public WidgetWindow(Settings settings,Action openSettings,Action exit,Action dragFinished)
    {
        Title="Keeper UsagePanel";Width=132;Height=132;WindowStyle=WindowStyle.None;ResizeMode=ResizeMode.NoResize;AllowsTransparency=true;Background=Brushes.Transparent;Topmost=true;ShowInTaskbar=false;ShowActivated=false;
        var border=new Border{CornerRadius=new CornerRadius(66),Background=Theme.Background,BorderBrush=Theme.Line,BorderThickness=new Thickness(2),Padding=new Thickness(10,13,10,10)};
        var stack=new StackPanel{HorizontalAlignment=HorizontalAlignment.Center};border.Child=stack;Content=border;
        var label=Theme.Text("今日 TOKENS",9,Theme.Muted);label.HorizontalAlignment=HorizontalAlignment.Center;stack.Children.Add(label);
        total.HorizontalAlignment=HorizontalAlignment.Center;total.Margin=new Thickness(0,1,0,0);stack.Children.Add(total);
        input.HorizontalAlignment=output.HorizontalAlignment=health.HorizontalAlignment=HorizontalAlignment.Center;
        stack.Children.Add(input);stack.Children.Add(output);health.Margin=new Thickness(0,4,0,0);stack.Children.Add(health);
        ContextMenu=new ContextMenu();var setup=new MenuItem{Header="连接设置"};setup.Click+=(_,_)=>openSettings();ContextMenu.Items.Add(setup);var quit=new MenuItem{Header="退出"};quit.Click+=(_,_)=>exit();ContextMenu.Items.Add(quit);
        SourceInitialized+=(_,_)=>Native.Configure(this,true);
        Loaded+=(_,_)=>Native.PlaceWidget(this,settings);
        Point? down=null;
        MouseLeftButtonDown+=(_,e)=>down=e.GetPosition(this);
        MouseMove+=(_,e)=>{if(down is not {} p||e.LeftButton!=MouseButtonState.Pressed)return;var q=e.GetPosition(this);if(Math.Abs(p.X-q.X)+Math.Abs(p.Y-q.Y)<5)return;down=null;Dragging=true;try{DragMove();}catch(InvalidOperationException){}finally{Dragging=false;var bounds=Native.Bounds(this);settings.X=bounds.Left;settings.Y=bounds.Top;Native.PlaceWidget(this,settings,true);bounds=Native.Bounds(this);settings.X=bounds.Left;settings.Y=bounds.Top;settings.Save();dragFinished();}};
        MouseLeftButtonUp+=(_,_)=>down=null;
    }
    internal void Display(JsonNode sample,int interval)
    {
        total.Text=J.Compact(J.N(sample,"today_tokens"));bool baseline=J.B(sample,"delta.baseline");
        input.Text="入 "+(baseline?"—":"+"+J.Compact(J.N(sample,"delta.input_tokens")));
        output.Text="出 "+(baseline?"—":"+"+J.Compact(J.N(sample,"delta.output_tokens")));
        health.Text=J.S(sample,"health.label");health.Foreground=health.Text=="健康"?Theme.Accent:health.Text=="波动"?Theme.Amber:health.Text=="异常"?Theme.Red:Theme.Muted;
        double elapsed=J.N(sample,"delta.seconds");
        ToolTip=$"北京时间今日：{J.Count(J.N(sample,"today_tokens"))} Token\n"+(baseline?"已建立采样基线，等待下一次采样。":$"相邻采样间隔 {elapsed:0.0} 秒\n输入 +{J.Count(J.N(sample,"delta.input_tokens"))} / 输出 +{J.Count(J.N(sample,"delta.output_tokens"))}"+(elapsed>interval*2+1?"\n包含断线或休眠期间累计增长的用量。":""))+"\n拖动移动 · 右键设置";
    }
    internal void State(string label,string message){health.Text=label;health.Foreground=Theme.Amber;input.Text="入 —";output.Text="出 —";ToolTip=message+"\n今日数字为最后一次成功采样结果。";}
}
