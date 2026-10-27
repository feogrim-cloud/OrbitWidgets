// Compiles with the .NET Framework csc supplied with Windows. The monitoring
// assembly is loaded from this helper's directory; no SDK is required at runtime.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Security.Principal;
using System.Text;
using System.Threading;

internal static class Program
{
    private static readonly CultureInfo Invariant = CultureInfo.InvariantCulture;
    private static readonly string BasePath = AppDomain.CurrentDomain.BaseDirectory;
    private static int parentId;
    private static bool? pawnIoInstalled;

    private sealed class Sample
    {
        public double? CpuTemp;
        public double? GpuTemp;
        public double? GpuUsage;
        public string GpuName = "";
        public int CpuScore = -1;
        public readonly List<GpuSample> Gpus = new List<GpuSample>();
    }

    private sealed class GpuSample
    {
        public string Name = "";
        public string Identifier = "";
        public double? Temperature;
        public double? Usage;
        public int TempScore = -1;
        public int LoadScore = -1;
    }

    private static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        bool once = false;
        string requestedGpuName = null;
        for (int i = 0; i < args.Length; i++)
        {
            if (args[i] == "--once") once = true;
            if (args[i] == "--parent" && i + 1 < args.Length)
                Int32.TryParse(args[++i], out parentId);
            if (args[i] == "--gpu-name")
                requestedGpuName = i + 1 < args.Length ? Identity(args[++i]) : "";
        }
        AppDomain.CurrentDomain.AssemblyResolve += ResolveLocalAssembly;
        object computer = null;
        try
        {
            string assemblyPath = Path.Combine(BasePath, "LibreHardwareMonitorLib.dll");
            if (!File.Exists(assemblyPath))
                throw new FileNotFoundException("Не найдена библиотека LibreHardwareMonitorLib.dll");
            Assembly library = Assembly.LoadFrom(assemblyPath);
            Type pawnType = library.GetType("LibreHardwareMonitor.PawnIo.PawnIo", false);
            if (pawnType != null)
            {
                PropertyInfo installed = pawnType.GetProperty("IsInstalled", BindingFlags.Public | BindingFlags.Static);
                if (installed != null)
                {
                    object installedValue = installed.GetValue(null, null);
                    if (installedValue is bool) pawnIoInstalled = (bool)installedValue;
                }
            }
            Type computerType = library.GetType("LibreHardwareMonitor.Hardware.Computer", true);
            computer = Activator.CreateInstance(computerType);
            Set(computer, "IsCpuEnabled", true);
            Set(computer, "IsGpuEnabled", true);
            Set(computer, "IsMotherboardEnabled", false);
            Set(computer, "IsMemoryEnabled", false);
            Set(computer, "IsStorageEnabled", false);
            Set(computer, "IsNetworkEnabled", false);
            Set(computer, "IsControllerEnabled", false);
            Invoke(computer, "Open");
            do
            {
                if (!ParentAlive()) break;
                Stopwatch interval = Stopwatch.StartNew();
                try
                {
                    IEnumerable hardware = Get(computer, "Hardware") as IEnumerable;
                    Sample sample = ReadSample(hardware, requestedGpuName);
                    string status = sample.CpuTemp.HasValue
                        ? "Температура ЦП доступна"
                        : (pawnIoInstalled == false
                            ? "Для температуры ЦП требуется модуль PawnIO из комплекта программы"
                            : (IsAdministrator()
                                ? "Датчик температуры ЦП недоступен. Проверьте доступ к PawnIO и поддержку оборудования"
                                : "Для температуры ЦП может потребоваться запуск от администратора"));
                    Emit(sample, status);
                }
                catch (Exception error)
                {
                    Emit(new Sample(), "Ошибка чтения датчиков: " + ErrorMessage(error));
                }
                if (once) break;
                // Both collectors sample at 1 Hz; main CPU and network use
                // a separate worker and cannot be held up by this work.
                int wait = Math.Max(0, 1000 - (int)interval.ElapsedMilliseconds);
                if (wait > 0) Thread.Sleep(wait);
            } while (true);
            return 0;
        }
        catch (Exception error)
        {
            Emit(new Sample(), "Модуль температур: " + ErrorMessage(error));
            return 1;
        }
        finally
        {
            if (computer != null)
                try { Invoke(computer, "Close"); } catch { }
        }
    }

    private static Assembly ResolveLocalAssembly(object sender, ResolveEventArgs args)
    {
        string filename = new AssemblyName(args.Name).Name + ".dll";
        string path = Path.Combine(BasePath, filename);
        return File.Exists(path) ? Assembly.LoadFrom(path) : null;
    }

    private static Sample ReadSample(IEnumerable hardware, string requestedGpuName)
    {
        Sample sample = new Sample();
        if (hardware != null)
            foreach (object item in hardware) ReadHardware(item, sample, null);
        SelectGpu(sample, requestedGpuName);
        return sample;
    }

    private static void ReadHardware(object hardware, Sample sample, GpuSample parentGpu)
    {
        if (hardware == null) return;
        string type;
        try { type = Convert.ToString(Get(hardware, "HardwareType"), Invariant) ?? ""; }
        catch { return; }
        bool cpu = String.Equals(type, "Cpu", StringComparison.OrdinalIgnoreCase);
        bool gpu = type.StartsWith("Gpu", StringComparison.OrdinalIgnoreCase);
        GpuSample adapter = cpu ? null : parentGpu;
        if (gpu)
        {
            // Keep an unavailable adapter in the identity list as well: a
            // failed update of one of two identically named GPUs must not
            // turn a formerly ambiguous requested name into a false match.
            string gpuName = "", gpuIdentifier = "";
            try
            {
                gpuName = Identity(Convert.ToString(Get(hardware, "Name"), Invariant));
                gpuIdentifier = Identity(Convert.ToString(Get(hardware, "Identifier"), Invariant));
            }
            catch { }
            if (adapter == null || !SameGpuSubtree(adapter.Identifier, gpuIdentifier))
            {
                adapter = new GpuSample { Name = gpuName, Identifier = gpuIdentifier };
                sample.Gpus.Add(adapter);
            }
        }
        // Sensor.Value can retain the preceding successful update. A failed
        // hardware update invalidates this whole subtree for the current sample.
        try
        {
            MethodInfo update = hardware.GetType().GetMethod("Update", Type.EmptyTypes);
            if (update == null) return;
            update.Invoke(hardware, null);
        }
        catch { return; }
        IEnumerable sensors = null;
        try { sensors = Get(hardware, "Sensors") as IEnumerable; } catch { }
        if (sensors != null)
        {
            foreach (object sensor in sensors)
            {
                object raw;
                string sensorType, name;
                try
                {
                    raw = Get(sensor, "Value");
                    sensorType = Convert.ToString(Get(sensor, "SensorType"), Invariant) ?? "";
                    name = (Convert.ToString(Get(sensor, "Name"), Invariant) ?? "").ToLowerInvariant();
                }
                catch { continue; }
                if (raw == null) continue;
                double value;
                try { value = Convert.ToDouble(raw, Invariant); } catch { continue; }
                if (Double.IsNaN(value) || Double.IsInfinity(value)) continue;
                // A missing PawnIO driver can surface as a zero-value CPU
                // sensor. Zero is unavailable on this desktop, not a reading.
                if (sensorType == "Temperature" && value > 1 && value < 150)
                {
                    if (cpu && pawnIoInstalled != false)
                    {
                        int score = CpuTemperatureScore(name);
                        if (score > sample.CpuScore)
                        {
                            sample.CpuScore = score;
                            sample.CpuTemp = value;
                        }
                    }
                    if (adapter != null)
                    {
                        int score = GpuTemperatureScore(name);
                        if (score > adapter.TempScore)
                        {
                            adapter.TempScore = score;
                            adapter.Temperature = value;
                        }
                    }
                }
                if (adapter != null && sensorType == "Load" && value >= 0 && value <= 100)
                {
                    int score = name == "gpu core" ? 100 : (name == "d3d 3d" ? 90 : (name == "gpu total" ? 80 : -1));
                    if (score > adapter.LoadScore)
                    {
                        adapter.LoadScore = score;
                        adapter.Usage = value;
                    }
                }
            }
        }
        IEnumerable children = null;
        try { children = Get(hardware, "SubHardware") as IEnumerable; } catch { }
        if (children != null)
            foreach (object child in children) ReadHardware(child, sample, adapter);
    }

    private static string Identity(string value)
    {
        if (value == null || value.Length > 512) return "";
        return value.Trim();
    }

    private static string GpuKey(GpuSample gpu)
    {
        return gpu.Identifier + "\n" + gpu.Name;
    }

    private static bool SameGpuSubtree(string parent, string child)
    {
        if (parent.Length == 0 || child.Length == 0) return false;
        return String.Equals(parent, child, StringComparison.OrdinalIgnoreCase)
            || child.StartsWith(parent.TrimEnd('/') + "/", StringComparison.OrdinalIgnoreCase);
    }

    private static void SelectGpu(Sample sample, string requestedGpuName)
    {
        GpuSample selected = null;
        if (requestedGpuName != null)
        {
            string requested = Identity(requestedGpuName);
            if (requested.Length == 0) return;
            foreach (GpuSample gpu in sample.Gpus)
            {
                if (!String.Equals(gpu.Name, requested, StringComparison.OrdinalIgnoreCase)) continue;
                if (selected != null) return; // A name cannot identify two physical adapters.
                selected = gpu;
            }
        }
        else
        {
            // Identity, not changing load/temperature scores, chooses a default.
            // Losing a reading therefore does not silently switch adapters.
            foreach (GpuSample gpu in sample.Gpus)
            {
                if (gpu.Name.Length == 0) continue;
                if (selected == null || StringComparer.OrdinalIgnoreCase.Compare(GpuKey(gpu), GpuKey(selected)) < 0)
                    selected = gpu;
            }
            if (selected != null)
            {
                int matches = 0;
                foreach (GpuSample gpu in sample.Gpus)
                    if (String.Equals(GpuKey(gpu), GpuKey(selected), StringComparison.OrdinalIgnoreCase)) matches++;
                if (matches != 1) return;
            }
        }
        if (selected == null) return;
        sample.GpuName = selected.Name;
        sample.GpuTemp = selected.Temperature;
        sample.GpuUsage = selected.Usage;
    }

    private static int CpuTemperatureScore(string name)
    {
        if (name.Contains("tctl/tdie")) return 110;
        if (name.Contains("tdie") && !name.Contains("ccd")) return 105;
        if (name.Contains("package")) return 100;
        if (name.Contains("core max")) return 95;
        if (name.Contains("core average")) return 90;
        if (name.Contains("tctl")) return 80;
        if (name.StartsWith("core #")) return 50;
        return -1;
    }

    private static int GpuTemperatureScore(string name)
    {
        if (name == "gpu core") return 100;
        if (name == "gpu package") return 95;
        if (name == "gpu temperature") return 90;
        // Hotspot and VRAM are different readings, so do not relabel them GPU Core.
        return -1;
    }

    private static void Emit(Sample value, string status)
    {
        string json = "{\"cpu_temp\":" + Number(value.CpuTemp)
            + ",\"gpu_temp\":" + Number(value.GpuTemp)
            + ",\"gpu_usage\":" + Number(value.GpuUsage)
            + ",\"gpu_name\":" + Quote(value.GpuName)
            + ",\"status\":" + Quote(status) + "}";
        try { Console.WriteLine(json); Console.Out.Flush(); }
        catch (IOException) { Environment.Exit(0); }
    }

    private static string Number(double? value)
    {
        return value.HasValue ? value.Value.ToString("0.###", Invariant) : "null";
    }

    private static string Quote(string value)
    {
        StringBuilder result = new StringBuilder("\"");
        foreach (char c in value ?? "")
        {
            if (c == '\\') result.Append("\\\\");
            else if (c == '"') result.Append("\\\"");
            else if (c == '\n') result.Append("\\n");
            else if (c == '\r') result.Append("\\r");
            else if (c == '\t') result.Append("\\t");
            else if (c < 32) result.Append("\\u" + ((int)c).ToString("x4", Invariant));
            else result.Append(c);
        }
        return result.Append('"').ToString();
    }

    private static object Get(object item, string property)
    {
        PropertyInfo info = item.GetType().GetProperty(property, BindingFlags.Instance | BindingFlags.Public);
        return info == null ? null : info.GetValue(item, null);
    }
    private static void Set(object item, string property, object value)
    {
        PropertyInfo info = item.GetType().GetProperty(property, BindingFlags.Instance | BindingFlags.Public);
        if (info != null && info.CanWrite) info.SetValue(item, value, null);
    }
    private static void Invoke(object item, string method)
    {
        MethodInfo info = item.GetType().GetMethod(method, Type.EmptyTypes);
        if (info != null) info.Invoke(item, null);
    }
    private static bool ParentAlive()
    {
        if (parentId <= 0) return true;
        try { using (Process parent = Process.GetProcessById(parentId)) return !parent.HasExited; }
        catch { return false; }
    }
    private static bool IsAdministrator()
    {
        try
        {
            using (WindowsIdentity identity = WindowsIdentity.GetCurrent())
                return new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator);
        }
        catch { return false; }
    }
    private static string ErrorMessage(Exception error)
    {
        while (error.InnerException != null) error = error.InnerException;
        return error.Message.Replace("\r", " ").Replace("\n", " ");
    }
}
