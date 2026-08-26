using System.Globalization;
using System.Text.Json.Nodes;

namespace KeeperUsagePanel.Core;

public sealed class PanelException(string state,string message):Exception(message){public string State{get;}=state;}
public sealed record UsageReading(DateOnly Day,long Input,long Output,long Requests,DateTimeOffset At);
public sealed record UsageTotals(long Input,long Output,long Requests,long Total=0,long CacheRead=0,long CacheWrite=0,long Reasoning=0);
public sealed record IntervalDelta(long Input,long Output,long Requests,double Seconds,bool Baseline,bool Reset);
public sealed class IntervalCounter
{
    public UsageReading? Previous{get;private set;}
    public IntervalDelta Accept(UsageReading current,UsageTotals? closedDays=null)
    {
        var previous=Previous;Previous=current;
        if(previous==null)return new(0,0,0,0,true,false);
        long input,output,requests;
        if(current.Day==previous.Day){input=current.Input-previous.Input;output=current.Output-previous.Output;requests=current.Requests-previous.Requests;}
        else if(current.Day>previous.Day&&closedDays!=null){input=closedDays.Input-previous.Input+current.Input;output=closedDays.Output-previous.Output+current.Output;requests=closedDays.Requests-previous.Requests+current.Requests;}
        else return new(0,0,0,0,true,true);
        double seconds=(current.At-previous.At).TotalSeconds;
        if(input<0||output<0||requests<0||seconds<0)return new(0,0,0,0,true,true);
        return new(input,output,requests,seconds,false,false);
    }
}
public static class Health
{
    public static string Label(long success,long failure){long total=success+failure;if(total==0)return "安静";if(failure>success)return "异常";double threshold=Math.Min(.99,.9+.045*Math.Max(0,Math.Log10(total/10.0)));return success/(double)total>=threshold?"健康":"波动";}
}
public static class J
{
    public static JsonNode? At(JsonNode? node,string path){foreach(var part in path.Split('.')){if(node is not JsonObject obj)return null;node=obj[part];}return node;}
    public static string S(JsonNode? n,string p,string fallback="—")=>At(n,p)?.ToString() is {Length:>0} s?s:fallback;
    public static double N(JsonNode? n,string p)=>double.TryParse(S(n,p,"0"),NumberStyles.Any,CultureInfo.InvariantCulture,out var v)?v:0;
    public static long L(JsonNode? n,string p)=>long.TryParse(S(n,p,"0"),NumberStyles.Integer,CultureInfo.InvariantCulture,out var v)?v:0;
    public static bool B(JsonNode? n,string p)=>bool.TryParse(S(n,p,"false"),out var v)&&v;
    public static IEnumerable<JsonNode?> Items(JsonNode? n,string p)=>At(n,p) is JsonArray a?a:[];
    public static string Count(double value)=>value.ToString("N0",CultureInfo.InvariantCulture);
    public static string Compact(double value)=>value>=1e9?(value/1e9).ToString("0.##",CultureInfo.InvariantCulture)+"B":value>=1e6?(value/1e6).ToString("0.##",CultureInfo.InvariantCulture)+"M":value>=1e3?(value/1e3).ToString("0.##",CultureInfo.InvariantCulture)+"K":Count(value);
    public static string Cost(JsonNode? n,string value="cost_usd",string available="cost_available")=>B(n,available)?"$"+N(n,value).ToString("0.0000",CultureInfo.InvariantCulture):"未配置 / 不可用";
    public static string Percent(double numerator,double denominator)=>denominator>0?(numerator/denominator*100).ToString("0.00",CultureInfo.InvariantCulture)+"%":"—";
    public static string Duration(double milliseconds)=>milliseconds>0?(milliseconds/1000).ToString("0.###",CultureInfo.InvariantCulture)+" s":"—";
    public static string Time(string? input)=>DateTimeOffset.TryParse(input,CultureInfo.InvariantCulture,DateTimeStyles.None,out var t)?t.ToOffset(TimeSpan.FromHours(8)).ToString("MM-dd HH:mm:ss"):"—";
}
public static class RangeQuery
{
    public static string Build(string range,string key="",DateTime? start=null,DateTime? end=null)
    {
        string q="range="+Uri.EscapeDataString(range);if(key.Length>0)q+="&api_key_id="+Uri.EscapeDataString(key);
        if(range=="custom"){if(start==null||end==null||start>end)throw new ArgumentException("请选择有效的开始和结束日期。");q+="&start="+start.Value.ToString("yyyy-MM-dd")+"&end="+end.Value.ToString("yyyy-MM-dd");}return q;
    }
    public static (DateTimeOffset start,DateTimeOffset end) Bounds(Dictionary<string,string> q,DateTimeOffset now)
    {
        now=now.ToOffset(TimeSpan.FromHours(8));var day=new DateTimeOffset(now.Date,TimeSpan.FromHours(8));
        return q.GetValueOrDefault("range","today") switch{
            "yesterday"=>(day.AddDays(-1),day),"7d"=>(now.AddDays(-7),now),"30d"=>(now.AddDays(-30),now),
            "month"=>(new DateTimeOffset(now.Year,now.Month,1,0,0,0,now.Offset),day.AddDays(1)),
            "custom"=>(new DateTimeOffset(DateTime.ParseExact(q["start"],"yyyy-MM-dd",CultureInfo.InvariantCulture),now.Offset),new DateTimeOffset(DateTime.ParseExact(q["end"],"yyyy-MM-dd",CultureInfo.InvariantCulture),now.Offset).AddDays(1)),
            _=>(day,day.AddDays(1))};
    }
}
