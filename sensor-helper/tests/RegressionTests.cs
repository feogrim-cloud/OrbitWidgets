// Reflection fixtures exercise the shipped helper code without loading the
// hardware library, opening a device, changing a profile or starting its Main.
using System;
using System.Collections;
using System.Reflection;

public sealed class FakeSensor
{
    public string SensorType { get; set; }
    public string Name { get; set; }
    public object Value { get; set; }
}

public sealed class FakeHardware
{
    public string HardwareType { get; set; }
    public string Name { get; set; }
    public string Identifier { get; set; }
    public IEnumerable Sensors { get; set; }
    public IEnumerable SubHardware { get; set; }
    public bool FailUpdate { get; set; }
    public int UpdateCount { get; private set; }
    public void Update()
    {
        UpdateCount++;
        if (FailUpdate) throw new InvalidOperationException("Synthetic hardware update failure");
    }
}

public sealed class MissingUpdateHardware
{
    public string HardwareType { get { return "GpuAmd"; } }
    public string Name { get { return "Missing update"; } }
    public string Identifier { get { return "/gpu/missing-update"; } }
    public IEnumerable Sensors { get { return new[] { new FakeSensor { SensorType = "Temperature", Name = "GPU Core", Value = 60.0 } }; } }
}

internal static class SensorHelperRegressionTests
{
    private static MethodInfo read;
    private static FieldInfo pawnIo;
    private static int count;

    private static object Value(object sample, string name)
    {
        return sample.GetType().GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance).GetValue(sample);
    }
    private static void Equal(object expected, object actual, string message)
    {
        if (!Object.Equals(expected, actual))
            throw new InvalidOperationException(message + ": expected " + (expected ?? "null") + ", got " + (actual ?? "null"));
    }
    private static FakeHardware Gpu(string name, string id, double temperature, string loadName, double load, bool fail)
    {
        return new FakeHardware { HardwareType = "GpuAmd", Name = name, Identifier = id, FailUpdate = fail,
            Sensors = new[] { new FakeSensor { SensorType = "Temperature", Name = "GPU Core", Value = temperature },
                new FakeSensor { SensorType = "Load", Name = loadName, Value = load } }, SubHardware = new object[0] };
    }
    private static FakeHardware Cpu(double temperature, bool fail)
    {
        return new FakeHardware { HardwareType = "Cpu", Name = "CPU", Identifier = "/cpu/0", FailUpdate = fail,
            Sensors = new[] { new FakeSensor { SensorType = "Temperature", Name = "CPU Package", Value = temperature } }, SubHardware = new object[0] };
    }
    private static object Read(string requested, params object[] hardware)
    {
        return read.Invoke(null, new object[] { hardware, requested });
    }
    private static void GpuEquals(object sample, string name, double? temperature, double? load)
    {
        Equal(name, Value(sample, "GpuName"), "GPU identity");
        Equal(temperature, Value(sample, "GpuTemp"), "GPU temperature");
        Equal(load, Value(sample, "GpuUsage"), "GPU load");
    }
    private static void Pass(string name)
    {
        count++;
        Console.WriteLine("PASS " + name);
    }
    private static void Main(string[] args)
    {
        if (args.Length != 1) throw new ArgumentException("Pass the compiled OrbitSensors.exe path");
        Assembly helper = Assembly.LoadFrom(args[0]);
        Type program = helper.GetType("Program", true);
        read = program.GetMethod("ReadSample", BindingFlags.NonPublic | BindingFlags.Static);
        pawnIo = program.GetField("pawnIoInstalled", BindingFlags.NonPublic | BindingFlags.Static);
        if (read == null || pawnIo == null) throw new InvalidOperationException("Helper regression entry points missing");

        var a = Gpu("Adapter A", "/gpu/0", 44, "D3D 3D", 10, false);
        var b = Gpu("Adapter B", "/gpu/1", 66, "GPU Core", 80, false);
        GpuEquals(Read(null, a, b), "Adapter A", 44, 10);
        GpuEquals(Read(null, b, a), "Adapter A", 44, 10);
        Pass("default GPU selection is coherent and independent of enumeration order");

        GpuEquals(Read("Adapter B", a, b), "Adapter B", 66, 80);
        GpuEquals(Read("  aDaPtEr B  ", b, a), "Adapter B", 66, 80);
        Pass("requested GPU owns name, temperature and load together");

        object missing = Read("Missing GPU", a, b, Cpu(67, false));
        GpuEquals(missing, "", null, null);
        Equal(67.0, Value(missing, "CpuTemp"), "Missing GPU must preserve CPU");
        GpuEquals(Read("", a, b), "", null, null);
        Pass("missing or empty requested name never falls back to another GPU");

        var duplicateA = Gpu("Duplicate", "/gpu/0", 44, "GPU Core", 10, false);
        var duplicateB = Gpu("Duplicate", "/gpu/1", 66, "GPU Core", 80, false);
        GpuEquals(Read("Duplicate", duplicateA, duplicateB), "", null, null);
        duplicateA.FailUpdate = true;
        GpuEquals(Read("Duplicate", duplicateA, duplicateB), "", null, null);
        Pass("requested names stay ambiguous even if one matching GPU fails");

        duplicateA.FailUpdate = false;
        GpuEquals(Read(null, duplicateB, duplicateA), "Duplicate", 44, 10);
        duplicateA.Identifier = duplicateB.Identifier;
        GpuEquals(Read(null, duplicateA, duplicateB), "", null, null);
        Pass("default identity is deterministic and duplicate identities are rejected");

        var failed = Gpu("Failing adapter", "/gpu/0", 44, "GPU Core", 50, true);
        GpuEquals(Read(null, failed), "Failing adapter", null, null);
        GpuEquals(Read("Failing adapter", failed, b), "Failing adapter", null, null);
        Pass("Update failure invalidates cached GPU values and does not switch adapters");

        var staleChild = Gpu("Child", "/gpu/child", 95, "GPU Core", 99, false);
        failed.SubHardware = new[] { staleChild };
        GpuEquals(Read(null, failed), "Failing adapter", null, null);
        Equal(0, staleChild.UpdateCount, "Failed parent subtree must be skipped");
        Pass("a failed parent cannot expose values from its cached subtree");

        var fresh = Gpu("Fresh adapter", "/gpu/0", 64, "D3D 3D", 50, false);
        fresh.SubHardware = new[] { Gpu("Failing child", "/gpu/child", 95, "GPU Core", 99, true) };
        GpuEquals(Read(null, fresh), "Fresh adapter", 64, 50);
        Pass("failed child cannot poison independently refreshed parent readings");

        object independent = Read(null, Cpu(72, true), Cpu(51, false), a);
        Equal(51.0, Value(independent, "CpuTemp"), "Healthy CPU must survive failed hardware");
        Equal(null, Value(Read(null, Cpu(72, true)), "CpuTemp"), "Cached CPU value must be invalidated");
        Pass("failed CPU update does not hide another healthy CPU or reuse cache");

        var container = new FakeHardware { HardwareType = "Unknown", Name = "Container", Sensors = new object[0], SubHardware = new object[] { a, b } };
        GpuEquals(Read("Adapter B", container), "Adapter B", 66, 80);
        Pass("nested physical GPUs are collected before a single target is selected");

        var nested = Gpu("Parent GPU", "/gpu/a", 44, "D3D 3D", 10, false);
        nested.SubHardware = new object[] { Gpu("Child GPU", "/gpu/b", 66, "GPU Core", 80, false) };
        GpuEquals(Read(null, nested), "Parent GPU", 44, 10);
        GpuEquals(Read("Child GPU", nested), "Child GPU", 66, 80);
        Pass("a distinct nested GPU cannot overwrite the parent's readings");

        var component = Gpu("Memory component", "/gpu/a/memory", 95, "GPU Core", 90, false);
        nested.SubHardware = new object[] { component };
        GpuEquals(Read(null, nested), "Parent GPU", 44, 90);
        nested.SubHardware = new object[] { new FakeHardware { HardwareType = "Cpu", Name = "CPU child", Identifier = "/cpu/child",
            Sensors = new[] { new FakeSensor { SensorType = "Load", Name = "GPU Core", Value = 99.0 } }, SubHardware = new object[0] } };
        GpuEquals(Read(null, nested), "Parent GPU", 44, 10);
        Pass("only identified GPU components share an adapter, explicit CPU hardware does not");

        GpuEquals(Read(null, new MissingUpdateHardware()), "Missing update", null, null);
        Pass("missing Update method cannot be treated as a fresh sensor read");

        var invalid = Gpu("Invalid values", "/gpu/invalid", Double.NaN, "GPU Core", Double.PositiveInfinity, false);
        GpuEquals(Read(null, invalid), "Invalid values", null, null);
        Equal(null, Value(Read(null, Cpu(0, false)), "CpuTemp"), "Missing temperature zero");
        object prior = pawnIo.GetValue(null);
        try
        {
            pawnIo.SetValue(null, false);
            Equal(null, Value(Read(null, Cpu(62, false)), "CpuTemp"), "PawnIO absence");
        }
        finally { pawnIo.SetValue(null, prior); }
        Pass("invalid numbers and unavailable CPU driver retain unknown readings");

        Console.WriteLine("Sensor helper regression tests passed: " + count);
    }
}
