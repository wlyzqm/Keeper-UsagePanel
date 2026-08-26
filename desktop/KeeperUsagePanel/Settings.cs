using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using Microsoft.Win32;

namespace KeeperUsagePanel;

internal sealed class Settings
{
    public const string RegistryPath=@"Software\KeeperUsagePanel";
    public string Endpoint{get;set;}="";
    public string Password{get;set;}="";
    public bool RememberPassword{get;set;}=true;
    public int PollSeconds{get;set;}=2;
    public bool AllowPrivateHttp{get;set;}
    public bool AutoStart{get;set;}
    public int? X{get;set;}
    public int? Y{get;set;}
    public static Settings Load()
    {
        var result=new Settings();using var key=Registry.CurrentUser.OpenSubKey(RegistryPath);if(key==null)return result;
        result.Endpoint=key.GetValue("Endpoint") as string??"";
        result.PollSeconds=Math.Clamp(key.GetValue("PollSeconds") is int poll?poll:2,1,60);
        result.RememberPassword=key.GetValue("RememberPassword") is int remember&&remember==1;
        result.AllowPrivateHttp=key.GetValue("AllowPrivateHttp") is int allow&&allow==1;
        result.AutoStart=key.GetValue("AutoStart") is int auto&&auto==1;
        result.X=key.GetValue("X") as int?;result.Y=key.GetValue("Y") as int?;
        if(result.RememberPassword&&key.GetValue("ProtectedPassword") is byte[] encrypted){try{result.Password=Encoding.UTF8.GetString(Dpapi.Transform(encrypted,false));}catch{result.Password="";}}
        return result;
    }
    public void Save()
    {
        using var key=Registry.CurrentUser.CreateSubKey(RegistryPath);
        key.SetValue("Endpoint",Endpoint,RegistryValueKind.String);key.SetValue("PollSeconds",PollSeconds,RegistryValueKind.DWord);
        key.SetValue("RememberPassword",RememberPassword?1:0,RegistryValueKind.DWord);key.SetValue("AllowPrivateHttp",AllowPrivateHttp?1:0,RegistryValueKind.DWord);key.SetValue("AutoStart",AutoStart?1:0,RegistryValueKind.DWord);
        if(X.HasValue)key.SetValue("X",X.Value,RegistryValueKind.DWord);if(Y.HasValue)key.SetValue("Y",Y.Value,RegistryValueKind.DWord);
        if(RememberPassword&&Password.Length>0)key.SetValue("ProtectedPassword",Dpapi.Transform(Encoding.UTF8.GetBytes(Password),true),RegistryValueKind.Binary);else key.DeleteValue("ProtectedPassword",false);
    }
    public void SetAutoStart(){using var key=Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Run");if(AutoStart)key.SetValue("KeeperUsagePanel","\""+Environment.ProcessPath+"\"");else key.DeleteValue("KeeperUsagePanel",false);}
}
internal static class Dpapi
{
    [StructLayout(LayoutKind.Sequential)]private struct Blob{public int Length;public IntPtr Data;}
    [DllImport("crypt32.dll",SetLastError=true,CharSet=CharSet.Unicode)]private static extern bool CryptProtectData(ref Blob input,string? description,IntPtr entropy,IntPtr reserved,IntPtr prompt,int flags,out Blob output);
    [DllImport("crypt32.dll",SetLastError=true)]private static extern bool CryptUnprotectData(ref Blob input,IntPtr description,IntPtr entropy,IntPtr reserved,IntPtr prompt,int flags,out Blob output);
    [DllImport("kernel32.dll")]private static extern IntPtr LocalFree(IntPtr ptr);
    public static byte[] Transform(byte[] data,bool encrypt){var input=new Blob{Length=data.Length,Data=Marshal.AllocHGlobal(data.Length)};try{Marshal.Copy(data,0,input.Data,data.Length);Blob output;bool ok=encrypt?CryptProtectData(ref input,"Keeper UsagePanel",IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,1,out output):CryptUnprotectData(ref input,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,1,out output);if(!ok)throw new InvalidOperationException("无法读取 Windows 用户凭据，请重新输入密码。");try{var result=new byte[output.Length];Marshal.Copy(output.Data,result,0,result.Length);return result;}finally{LocalFree(output.Data);}}finally{Marshal.FreeHGlobal(input.Data);}}
}
internal sealed class SettingsWindow:Window
{
    public SettingsWindow(Settings settings,Action saved)
    {
        Title="Keeper · 首次连接 / 设置";Width=475;Height=590;ResizeMode=ResizeMode.NoResize;WindowStartupLocation=WindowStartupLocation.CenterScreen;Theme.Apply(this);
        var stack=new StackPanel{Margin=new Thickness(26)};Content=new ScrollViewer{Content=stack,VerticalScrollBarVisibility=ScrollBarVisibility.Auto};
        stack.Children.Add(Theme.Text("连接你的 Keeper",24,Theme.Accent));Theme.Note(stack,"直接连接已有 Keeper，不安装远端服务。地址与偏好保存在当前用户注册表。");
        Theme.Heading(stack,"Keeper 地址");var endpoint=new TextBox{Text=settings.Endpoint,Height=30};stack.Children.Add(endpoint);Theme.Note(stack,"填写完整页面地址，例如 https://keeper.example/usage");
        Theme.Heading(stack,"Keeper 登录密码");var password=new PasswordBox{Height=30,Password=settings.Password};stack.Children.Add(password);
        var remember=new CheckBox{Content="记住密码（Windows DPAPI 加密保存）",IsChecked=settings.RememberPassword,Margin=new Thickness(0,12,0,8),Foreground=Theme.Ink};stack.Children.Add(remember);
        var privateHttp=new CheckBox{Content="允许受保护专网内的 HTTP 连接",IsChecked=settings.AllowPrivateHttp,Margin=new Thickness(0,2,0,8),Foreground=Theme.Ink};stack.Children.Add(privateHttp);
        var auto=new CheckBox{Content="登录 Windows 后启动",IsChecked=settings.AutoStart,Foreground=Theme.Ink,Margin=new Thickness(0,2,0,10)};stack.Children.Add(auto);
        stack.Children.Add(Theme.Text("刷新间隔（秒，1–60）",11,Theme.Muted));var poll=new TextBox{Text=settings.PollSeconds.ToString(),Width=60,HorizontalAlignment=HorizontalAlignment.Left};stack.Children.Add(poll);
        var error=Theme.Text("",11,Theme.Red);error.Margin=new Thickness(0,10,0,8);stack.Children.Add(error);
        stack.Children.Add(Theme.Button("保存并连接",()=>{
            try{
                if(!Uri.TryCreate(endpoint.Text.Trim(),UriKind.Absolute,out var uri)||(uri.Scheme!="https"&&uri.Scheme!="http"))throw new Exception("请输入 HTTP/HTTPS Keeper 地址。");
                if(!string.IsNullOrEmpty(uri.UserInfo)||!string.IsNullOrEmpty(uri.Query)||!string.IsNullOrEmpty(uri.Fragment))throw new Exception("请使用不带账号、查询参数或 # 片段的 Keeper 地址。");
                if(uri.Scheme=="http"&&!uri.IsLoopback&&privateHttp.IsChecked!=true)throw new Exception("非本机 HTTP 连接需要确认专网选项；公网请使用 HTTPS。");
                if(!int.TryParse(poll.Text,out var interval)||interval<1||interval>60)throw new Exception("刷新间隔必须为 1–60 秒。");
                settings.Endpoint=endpoint.Text.Trim().TrimEnd('/');settings.Password=password.Password;settings.RememberPassword=remember.IsChecked==true;settings.PollSeconds=interval;settings.AllowPrivateHttp=privateHttp.IsChecked==true;settings.AutoStart=auto.IsChecked==true;settings.SetAutoStart();settings.Save();saved();Close();
            }catch(Exception ex){error.Text=ex.Message;}
        },true));
    }
}
