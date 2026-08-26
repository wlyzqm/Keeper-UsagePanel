using System;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using Screen=System.Windows.Forms.Screen;

namespace KeeperUsagePanel;

internal static class Native
{
    [StructLayout(LayoutKind.Sequential)]internal struct Point{public int X,Y;}
    [StructLayout(LayoutKind.Sequential)]internal struct Rect{public int Left,Top,Right,Bottom;public int Width=>Right-Left;public int Height=>Bottom-Top;public bool Contains(Point p)=>p.X>=Left&&p.X<Right&&p.Y>=Top&&p.Y<Bottom;}
    [DllImport("user32.dll")]internal static extern bool GetCursorPos(out Point p);
    [DllImport("user32.dll")]private static extern bool GetWindowRect(IntPtr hwnd,out Rect r);
    [DllImport("user32.dll",EntryPoint="GetWindowLongW")]private static extern int GetWindowLong(IntPtr hwnd,int index);
    [DllImport("user32.dll",EntryPoint="SetWindowLongW")]private static extern int SetWindowLong(IntPtr hwnd,int index,int value);
    [DllImport("user32.dll")]private static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int cx,int cy,uint flags);
    [DllImport("user32.dll")]private static extern uint GetDpiForWindow(IntPtr hwnd);
    internal static IntPtr Handle(Window window)=>new WindowInteropHelper(window).Handle;
    internal static Rect Bounds(Window window){GetWindowRect(Handle(window),out var r);return r;}
    internal static void Configure(Window window,bool noActivate)
    {
        var hwnd=Handle(window);var flags=GetWindowLong(hwnd,-20)|0x80;if(noActivate)flags|=0x08000000;SetWindowLong(hwnd,-20,flags);
    }
    internal static void PlaceWidget(Window widget,Settings settings,bool snap=false)
    {
        var hwnd=Handle(widget);var current=Bounds(widget);var screen=settings.X.HasValue?Screen.FromPoint(new System.Drawing.Point(settings.X.Value,settings.Y??0)):Screen.PrimaryScreen!;var area=screen.WorkingArea;
        int x=settings.X??(area.Right-current.Width-24),y=settings.Y??(area.Bottom-current.Height-24);
        x=Math.Clamp(x,area.Left,Math.Max(area.Left,area.Right-current.Width));y=Math.Clamp(y,area.Top,Math.Max(area.Top,area.Bottom-current.Height));
        if(snap){if(x-area.Left<28)x=area.Left+4;else if(area.Right-x-current.Width<28)x=area.Right-current.Width-4;}
        SetWindowPos(hwnd,IntPtr.Zero,x,y,0,0,0x15);
    }
    internal static void PlacePanel(Window panel,Window widget)
    {
        var b=Bounds(widget);var area=Screen.FromHandle(Handle(widget)).WorkingArea;double scale=GetDpiForWindow(Handle(widget))/96.0;
        int width=Math.Min((int)(panel.Width*scale),area.Width-16),height=Math.Min((int)(panel.Height*scale),area.Height-16),gap=(int)(8*scale);
        int x=b.Right+gap;if(x+width>area.Right)x=b.Left-gap-width;x=Math.Clamp(x,area.Left,Math.Max(area.Left,area.Right-width));
        int y=Math.Clamp(b.Top-40,area.Top,Math.Max(area.Top,area.Bottom-height));
        SetWindowPos(Handle(panel),new IntPtr(-1),x,y,width,height,0x10);
    }
    internal static bool InBall(Window widget,Point p){var r=Bounds(widget);if(r.Width==0||r.Height==0)return false;double dx=(p.X-(r.Left+r.Width/2.0))/(r.Width/2.0),dy=(p.Y-(r.Top+r.Height/2.0))/(r.Height/2.0);return dx*dx+dy*dy<=1;}
    internal static bool InBridge(Window widget,Window panel,Point p){var a=Bounds(widget);var b=Bounds(panel);var gap=new Rect{Left=Math.Min(a.Right,b.Right),Right=Math.Max(a.Left,b.Left),Top=Math.Max(a.Top,b.Top),Bottom=Math.Min(a.Bottom,b.Bottom)};return gap.Right>gap.Left&&gap.Contains(p);}
}
