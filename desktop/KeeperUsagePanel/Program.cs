using System;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Threading;
using KeeperUsagePanel.Core;
using Microsoft.Win32;
using Forms=System.Windows.Forms;

namespace KeeperUsagePanel;

internal static class Program
{
    [STAThread]public static void Main()
    {
        using var mutex=new Mutex(true,@"Local\KeeperUsagePanel",out bool first);if(!first)return;
        Forms.Application.SetHighDpiMode(Forms.HighDpiMode.PerMonitorV2);
        var app=new Application{ShutdownMode=ShutdownMode.OnExplicitShutdown};
        app.DispatcherUnhandledException+=(_,e)=>{MessageBox.Show("程序遇到错误："+e.Exception.Message,"Keeper UsagePanel");e.Handled=true;};
        var controller=new Controller(app);app.Exit+=(_,_)=>controller.Dispose();controller.Start();app.Run();
    }
}
internal sealed class Controller:IDisposable
{
    private readonly Application app;
    private readonly Settings settings=Settings.Load();
    private readonly WidgetWindow ball;
    private readonly DetailWindow panel;
    private readonly Forms.NotifyIcon tray;
    private readonly DispatcherTimer hover=new(){Interval=TimeSpan.FromMilliseconds(80)};
    private CancellationTokenSource? polling;
    private PanelClient? client;
    private SettingsWindow? settingsWindow;
    private DateTime? entered,left;
    private bool suspended,disposed;
    private DateTime lastGood=DateTime.MinValue;
    public Controller(Application app)
    {
        this.app=app;ball=new WidgetWindow(settings,OpenSettings,app.Shutdown,()=>panel?.Hide());
        panel=new DetailWindow(()=>client,OpenSettings);
        tray=new Forms.NotifyIcon{Text="Keeper UsagePanel",Icon=System.Drawing.SystemIcons.Information,Visible=true};
        var menu=new Forms.ContextMenuStrip();menu.Items.Add("显示 / 隐藏悬浮球",null,(_,_)=>{if(ball.IsVisible){panel.Hide();ball.Hide();}else{ball.Show();Native.PlaceWidget(ball,settings);}});menu.Items.Add("连接设置",null,(_,_)=>OpenSettings());menu.Items.Add("退出",null,(_,_)=>app.Shutdown());tray.ContextMenuStrip=menu;tray.DoubleClick+=(_,_)=>{ball.Show();Native.PlaceWidget(ball,settings);};
        hover.Tick+=(_,_)=>HoverTick();
        SystemEvents.PowerModeChanged+=PowerChanged;SystemEvents.SessionSwitch+=SessionChanged;SystemEvents.DisplaySettingsChanged+=DisplaysChanged;
    }
    public void Start(){ball.Show();panel.Owner=ball;hover.Start();if(string.IsNullOrWhiteSpace(settings.Endpoint)||(settings.Password.Length==0&&!settings.RememberPassword))OpenSettings();else Connect();}
    private void Connect()
    {
        polling?.Cancel();client?.Dispose();client=null;panel.Hide();
        if(string.IsNullOrWhiteSpace(settings.Endpoint))return;
        try{client=new PanelClient(settings.Endpoint,settings.Password);polling=new CancellationTokenSource();_=PollAsync(client,polling.Token);}catch(Exception ex){ball.State("需设置",ex.Message);}
    }
    private async Task PollAsync(PanelClient source,CancellationToken token)
    {
        using var timer=new PeriodicTimer(TimeSpan.FromSeconds(Math.Clamp(settings.PollSeconds,1,60)));
        do
        {
            if(token.IsCancellationRequested)return;
            if(!suspended)
            {
                try{var sample=await source.SampleAsync(token);if(token.IsCancellationRequested||!ReferenceEquals(source,client))return;lastGood=DateTime.UtcNow;ball.Display(sample,settings.PollSeconds);panel.SetSample(sample,settings.PollSeconds);}
                catch(OperationCanceledException) when(token.IsCancellationRequested){return;}
                catch(Exception ex){if(token.IsCancellationRequested)return;var state=ex is PanelException p?p.State:"离线";ball.State(state,"采样失败。"+(lastGood==DateTime.MinValue?"尚无成功采样。":"上次成功："+lastGood.ToLocalTime().ToString("HH:mm:ss")));panel.SetConnectionState(state);}
            }
            try{if(!await timer.WaitForNextTickAsync(token))return;}catch(OperationCanceledException){return;}
        }while(true);
    }
    private void HoverTick()
    {
        if(!ball.IsVisible||suspended||ball.Dragging||settingsWindow?.IsVisible==true){panel.Hide();entered=left=null;return;}
        if(lastGood!=DateTime.MinValue&&(DateTime.UtcNow-lastGood).TotalSeconds>Math.Max(10,settings.PollSeconds*3))panel.SetConnectionState("数据过期");
        Native.GetCursorPos(out var p);bool onBall=Native.InBall(ball,p);
        if(!panel.IsVisible){if(onBall){entered??=DateTime.UtcNow;if((DateTime.UtcNow-entered.Value).TotalMilliseconds>=180){panel.Open();Native.PlacePanel(panel,ball);entered=null;}}else entered=null;return;}
        bool inside=onBall||Native.Bounds(panel).Contains(p)||Native.InBridge(ball,panel,p)||ball.MenuOpen||panel.HasOpenPopup;
        if(inside)left=null;else{left??=DateTime.UtcNow;if((DateTime.UtcNow-left.Value).TotalMilliseconds>=280){panel.Hide();left=null;}}
    }
    private void OpenSettings(){panel.Hide();if(settingsWindow?.IsVisible==true){settingsWindow.Activate();return;}settingsWindow=new SettingsWindow(settings,Connect);settingsWindow.Closed+=(_,_)=>settingsWindow=null;settingsWindow.Show();}
    private void PowerChanged(object sender,PowerModeChangedEventArgs e)=>app.Dispatcher.BeginInvoke(()=>{suspended=e.Mode==PowerModes.Suspend;if(suspended){panel.Hide();ball.State("已暂停","系统休眠，恢复后继续采样。");}});
    private void SessionChanged(object sender,SessionSwitchEventArgs e)=>app.Dispatcher.BeginInvoke(()=>{if(e.Reason==SessionSwitchReason.SessionLock){suspended=true;panel.Hide();}else if(e.Reason==SessionSwitchReason.SessionUnlock)suspended=false;});
    private void DisplaysChanged(object? sender,EventArgs e)=>app.Dispatcher.BeginInvoke(()=>{Native.PlaceWidget(ball,settings);if(panel.IsVisible)Native.PlacePanel(panel,ball);});
    public void Dispose(){if(disposed)return;disposed=true;polling?.Cancel();client?.Dispose();hover.Stop();panel.Stop();tray.Visible=false;tray.Dispose();SystemEvents.PowerModeChanged-=PowerChanged;SystemEvents.SessionSwitch-=SessionChanged;SystemEvents.DisplaySettingsChanged-=DisplaysChanged;}
}
