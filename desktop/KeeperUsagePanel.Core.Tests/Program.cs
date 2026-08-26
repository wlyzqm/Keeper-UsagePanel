using System.Net;
using System.Text.Json.Nodes;
using KeeperUsagePanel.Core;

int checks=0;
void Check(bool condition,string label){if(!condition)throw new Exception(label);checks++;}
var now=new DateTimeOffset(2026,8,26,12,0,0,TimeSpan.FromHours(8));
var date=new DateOnly(2026,8,26);var counter=new IntervalCounter();
Check(counter.Accept(new(date,100,10,1,now)).Baseline,"First sample establishes baseline");
var delta=counter.Accept(new(date,180,25,3,now.AddSeconds(2)));
Check(delta.Input==80&&delta.Output==15&&delta.Requests==2&&delta.Seconds==2,"Two-second cumulative difference");
delta=counter.Accept(new(date,180,25,3,now.AddSeconds(4)));
Check(delta.Input==0&&!delta.Baseline,"Empty interval is zero");
delta=counter.Accept(new(date.AddDays(1),30,4,1,now.AddDays(1)),new UsageTotals(200,30,4));
Check(delta.Input==50&&delta.Output==9&&delta.Requests==2,"Midnight adds final old-day increment and new day");
Check(counter.Accept(new(date.AddDays(1),1,1,1,now.AddDays(1).AddSeconds(2))).Reset,"Counter rollback rebaselines");
Check(counter.Accept(new(date.AddDays(2),100,10,1,now.AddDays(2))).Reset,"Missing cross-day bridge never fabricates a delta");
foreach(var c in new[]{(0L,0L,"安静"),(9L,1L,"健康"),(90L,10L,"波动"),(95L,5L,"健康"),(1L,1L,"波动"),(1L,2L,"异常")})Check(Health.Label(c.Item1,c.Item2)==c.Item3,"Keeper health threshold");

var handler=new FakeKeeper();
using var client=new PanelClient("https://example.invalid/usage","local-test-password",handler,()=>now);
var first=await client.SampleAsync();Check(J.B(first,"delta.baseline"),"HTTP baseline");
now=now.AddSeconds(2);
try{await client.SampleAsync();throw new Exception("Expected failure");}catch(PanelException){}
now=now.AddSeconds(2);var afterFailure=await client.SampleAsync();
Check(J.L(afterFailure,"delta.input_tokens")==60&&J.L(afterFailure,"delta.output_tokens")==6&&J.N(afterFailure,"delta.seconds")==4,"Failed HTTP poll preserves baseline");
Check(J.S(afterFailure,"health.label")=="波动","Direct credential health aggregation");
var summary=await client.GetViewAsync("v1/summary?range=30d&api_key_id=42");
Check(J.L(summary,"activity.output_tokens")==70,"Summary uses exact analysis token buckets, not activity grid range");
Check(handler.Paths.Any(x=>x.Contains("usage/analysis?range=30d&api_key_id=42")),"Key filtering reaches Keeper");
await client.GetViewAsync("v1/accounts");await client.GetViewAsync("v1/accounts/1/quota");
var errors=await client.GetViewAsync("v1/accounts/1/errors?range=today&api_key_id=42");
Check(J.Items(errors,"events").Count()==1&&J.L(errors,"total_count")==1&&!J.B(errors,"has_more"),"Errors filter current page without inventing range totals");
Check(!handler.Paths.Any(x=>x.Contains("quota/refresh")||x.Contains("56411")),"No quota refresh or adapter calls");
Check(handler.Logins==1,"Keeps one authenticated session");
Check(RangeQuery.Build("30d","42")=="range=30d&api_key_id=42","Range encoding");
Check(J.Time("2026-08-26T18:00:00Z")=="08-27 02:00:00","Beijing timezone");
Check(J.Percent(0,0)=="—"&&J.Compact(12500)=="12.5K","Metric formatting");
Check(J.L(JsonNode.Parse("{\"id\":9007199254740993}"),"id")==9007199254740993,"Integer precision");
Console.WriteLine($"PASS: {checks} checks — direct Keeper routes, login, interval/midnight/failure/reset, health, filters, errors and formatting.");

sealed class FakeKeeper:HttpMessageHandler
{
    public List<string> Paths{get;}=[];
    public int Logins{get;private set;}
    private int samples;
    protected override Task<HttpResponseMessage> SendAsync(HttpRequestMessage request,CancellationToken ct)
    {
        string path=request.RequestUri!.PathAndQuery;Paths.Add(path);string json;
        if(!path.StartsWith("/usage/api/v1/"))throw new Exception("Not a direct Keeper route");
        if(path.EndsWith("auth/login")){Logins++;if(request.Method!=HttpMethod.Post||!request.Headers.Contains("X-CPA-Usage-Keeper-Request"))throw new Exception("Login intent header");json="{}";}
        else if(path.Contains("usage/activity"))
        {
            samples++;if(samples==2)return Task.FromResult(new HttpResponseMessage(HttpStatusCode.ServiceUnavailable){Content=new StringContent("{}")});
            json=$$"""{"timezone":"Asia/Shanghai","window_start":"2026-08-26T00:00:00+08:00","input_tokens":{{(samples==1?100:160)}},"output_tokens":{{(samples==1?10:16)}},"total_tokens":{{(samples==1?110:176)}},"total_success":2,"total_failure":0}""";
        }
        else if(path.Contains("usage/identities/page"))json="""{"total_pages":1,"total_count":1,"identities":[{"id":"1","identity":"auth-a","type":"codex","credential_health":{"total_success":90,"total_failure":10}}]}""";
        else if(path.Contains("usage/overview"))json="""{"usage":{"total_tokens":1070,"total_requests":2},"summary":{"input_tokens":1000}}""";
        else if(path.Contains("usage/analysis"))json="""{"token_usage":[{"input_tokens":1000,"output_tokens":70,"total_tokens":1070,"requests":2}]}""";
        else if(path.Contains("quota/cache")){if(request.Method!=HttpMethod.Post)throw new Exception("Quota cache read is POST");json="{\"items\":[]}";}
        else if(path.Contains("/errors"))json="""{"has_more":true,"next_cursor":"older","events":[{"timestamp":"2026-08-26T10:00:00+08:00"},{"timestamp":"2026-08-25T10:00:00+08:00"}]}""";
        else throw new Exception("Unexpected route "+path);
        return Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK){Content=new StringContent(json)});
    }
}
