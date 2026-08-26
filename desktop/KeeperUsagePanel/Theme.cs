using System;
using System.Data;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Media;

namespace KeeperUsagePanel;

internal static class Theme
{
    public static readonly Brush Background = Brush("#101B1B"), Surface = Brush("#192929"), Line = Brush("#304342"), Ink = Brush("#ECF3E8"), Muted = Brush("#9DB4AA"), Accent = Brush("#C4EA9B"), Amber = Brush("#EAC789"), Red = Brush("#EB8B80");
    public static Brush Brush(string hex) { var b=new SolidColorBrush((Color)ColorConverter.ConvertFromString(hex)); b.Freeze();return b; }
    public static TextBlock Text(string text,double size=13,Brush? color=null) => new() { Text=text,FontSize=size,Foreground=color??Ink,FontFamily=new FontFamily("Microsoft YaHei UI"),TextWrapping=TextWrapping.Wrap,VerticalAlignment=VerticalAlignment.Center };
    public static TextBlock Number(string text,double size=28,Brush? color=null) { var t=Text(text,size,color);t.FontFamily=new FontFamily("Bahnschrift");return t; }
    public static Button Button(string title,Action action,bool primary=false)
    {
        var b=new Button {Content=title,Padding=new Thickness(12,8,12,8),Margin=new Thickness(0,0,6,0),Background=primary?Accent:Surface,Foreground=primary?Background:Ink,BorderBrush=Line,BorderThickness=new Thickness(1),Cursor=System.Windows.Input.Cursors.Hand,FontSize=12};
        var border=new FrameworkElementFactory(typeof(Border));border.SetValue(Border.CornerRadiusProperty,new CornerRadius(7));border.SetBinding(Border.BackgroundProperty,new Binding("Background"){RelativeSource=new RelativeSource(RelativeSourceMode.TemplatedParent)});border.SetBinding(Border.BorderBrushProperty,new Binding("BorderBrush"){RelativeSource=new RelativeSource(RelativeSourceMode.TemplatedParent)});border.SetValue(Border.BorderThicknessProperty,new Thickness(1));
        var content=new FrameworkElementFactory(typeof(ContentPresenter));content.SetBinding(ContentPresenter.MarginProperty,new Binding("Padding"){RelativeSource=new RelativeSource(RelativeSourceMode.TemplatedParent)});content.SetValue(ContentPresenter.HorizontalAlignmentProperty,HorizontalAlignment.Center);content.SetValue(ContentPresenter.VerticalAlignmentProperty,VerticalAlignment.Center);border.AppendChild(content);
        b.Template=new ControlTemplate(typeof(Button)){VisualTree=border};b.MouseEnter+=(_,_)=>b.Opacity=.8;b.MouseLeave+=(_,_)=>b.Opacity=1;b.Click+=(_,_)=>action();return b;
    }
    public static Border Card(string title,string value,string? note=null)
    {
        var s=new StackPanel();s.Children.Add(Text(title,11,Muted));var n=Number(value,24);n.Margin=new Thickness(0,8,0,2);s.Children.Add(n);if(note!=null)s.Children.Add(Text(note,10,Muted));
        return new Border{Background=Surface,BorderBrush=Line,BorderThickness=new Thickness(1),CornerRadius=new CornerRadius(10),Padding=new Thickness(14),Margin=new Thickness(0,0,8,8),Child=s};
    }
    public static void Cards(Panel target,params (string title,string value,string? note)[] metrics)
    {
        var grid=new System.Windows.Controls.Primitives.UniformGrid{Columns=2,Margin=new Thickness(0,8,0,4)};
        foreach(var m in metrics)grid.Children.Add(Card(m.title,m.value,m.note));target.Children.Add(grid);
    }
    public static void Heading(Panel target,string text) {var t=Text(text,15,Accent);t.Margin=new Thickness(0,18,0,10);target.Children.Add(t);}
    public static void Note(Panel target,string text) {var t=Text(text,11,Muted);t.Margin=new Thickness(0,6,0,8);target.Children.Add(t);}
    public static void Table(Panel target,string[] columns,System.Collections.Generic.IEnumerable<string[]> rows)
    {
        var table=new DataTable();foreach(var c in columns)table.Columns.Add(c);foreach(var r in rows)table.Rows.Add(r.Cast<object>().ToArray());
        if(table.Rows.Count==0){Note(target,"此范围暂无记录");return;}
        var grid=new DataGrid{ItemsSource=table.DefaultView,AutoGenerateColumns=true,IsReadOnly=true,CanUserAddRows=false,CanUserDeleteRows=false,CanUserReorderColumns=false,HeadersVisibility=DataGridHeadersVisibility.Column,GridLinesVisibility=DataGridGridLinesVisibility.Horizontal,HorizontalGridLinesBrush=Line,Background=Surface,Foreground=Ink,RowBackground=Surface,AlternatingRowBackground=Background,BorderThickness=new Thickness(0),FontSize=11,RowHeight=32,MaxHeight=340,MinHeight=70,ColumnWidth=new DataGridLength(110),SelectionUnit=DataGridSelectionUnit.FullRow};
        var header=new Style(typeof(System.Windows.Controls.Primitives.DataGridColumnHeader));header.Setters.Add(new Setter(Control.BackgroundProperty,Background));header.Setters.Add(new Setter(Control.ForegroundProperty,Muted));header.Setters.Add(new Setter(Control.PaddingProperty,new Thickness(8)));grid.ColumnHeaderStyle=header;
        var cell=new Style(typeof(DataGridCell));cell.Setters.Add(new Setter(Control.BorderThicknessProperty,new Thickness(0)));cell.Setters.Add(new Setter(Control.PaddingProperty,new Thickness(5)));var selected=new Trigger{Property=DataGridCell.IsSelectedProperty,Value=true};selected.Setters.Add(new Setter(Control.BackgroundProperty,Line));selected.Setters.Add(new Setter(Control.ForegroundProperty,Ink));cell.Triggers.Add(selected);grid.CellStyle=cell;
        target.Children.Add(grid);
    }
    public static ComboBox Combo(double width=140) => new(){Width=width,Height=32,Margin=new Thickness(0,0,8,0),Foreground=Brush("#172421"),Background=Brush("#EDF2E8"),VerticalContentAlignment=VerticalAlignment.Center};
    public static void Apply(Window w) {w.Background=Background;w.Foreground=Ink;w.FontFamily=new FontFamily("Microsoft YaHei UI");w.FontSize=13;}
}
