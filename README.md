<h1 align="center">Orbit Widgets</h1>

<p align="center">Circular performance widgets for your Windows desktop.</p>
<p align="center"><strong>English</strong> · <a href="README.ru.md">Русский</a></p>

Orbit Widgets displays CPU and GPU load and temperature, memory use, occupied drive space, current drive read/write speeds and network traffic in transparent desktop circles. The **dual-ring** design combines related readings without a surrounding panel. The current application interface is in Russian.

**[Download the latest release](https://github.com/feogrim-cloud/OrbitWidgets/releases/latest)** · [Build from source](docs/BUILD.md) · [Report an issue](https://github.com/feogrim-cloud/OrbitWidgets/issues)

[![Horizontal widgets in the neon theme](assets/screenshots/hero-horizontal.png)](assets/screenshots/hero-horizontal.png)

*The actual interface rendered with demonstration readings. Neutral backdrops make the transparent widgets visible; they are not an application panel. Click any image to view it at full size.*

## Download

For **Windows 10 version 2004 or newer and Windows 11, Intel/AMD x64**.

| Package | Download | Use |
| --- | --- | --- |
| Installer | [OrbitWidgets-Setup-x64.exe](https://github.com/feogrim-cloud/OrbitWidgets/releases/latest/download/OrbitWidgets-Setup-x64.exe) | Install the application, optionally install the CPU sensor driver, and create a desktop shortcut. |
| Portable ZIP | [OrbitWidgets-Windows-x64.zip](https://github.com/feogrim-cloud/OrbitWidgets/releases/latest/download/OrbitWidgets-Windows-x64.zip) | Extract the entire archive to a permanent folder and run `OrbitWidgets.exe`. |

Keep the `sensors` folder beside the application. No development tools are needed for the ready-made packages. The sensor helper uses .NET Framework 4.7.2 or newer. Orbit Widgets executables are currently unsigned; the separately bundled official PawnIO driver installer is signed by its publisher. Release assets include a SHA-256 checksum manifest.

Windows x86, Windows ARM64 and Linux packages are not provided. There is no automatic updater.

## Interface gallery

**Light theme**

[![Horizontal widgets in the light theme](assets/screenshots/theme-light.png)](assets/screenshots/theme-light.png)

<table>
  <tr><th>Vertical placement at the right edge</th><th>Metric and appearance settings</th></tr>
  <tr>
    <td align="center"><a href="assets/screenshots/vertical.png"><img src="assets/screenshots/vertical.png" height="420" alt="Vertical widgets at the right edge"></a></td>
    <td align="center"><a href="assets/screenshots/settings.png"><img src="assets/screenshots/settings.png" height="420" alt="Orbit Widgets settings, including elevated startup"></a></td>
  </tr>
</table>

**Drive capacity and live speed at 83 and 128 px**

[![Drive capacity and read/write speed modes](assets/screenshots/disk-speed.png)](assets/screenshots/disk-speed.png)

**RAM use, percentage and available memory at 83 and 128 px**

[![RAM circle with occupied percentage and available physical memory](assets/screenshots/ram-memory.png)](assets/screenshots/ram-memory.png)

## Features

- **CPU and GPU:** load on the outer ring, temperature on the inner ring. The Russian GPU label is **ГП**.
- **Memory:** occupied percentage in the header, used RAM in the center, total capacity below it and available physical memory on the bottom line of the same circle. The outer ring shows occupied memory; the tooltip also lists all capacities with units. Available memory includes the reclaimable part of the Windows cache. All RAM values use the same snapshot, updated every 500 ms.
- **Drives:** a **Объём / Скорость** (Capacity / Speed) switch for occupied/total capacity or current read/write speeds per drive letter, with individual visibility switches. The outer ring always shows occupied space; in Speed mode the inner ring shows combined read/write throughput against an adjustable scale, initially 1000 MB/s. This scale is not a disk utilization percentage.
- **Network:** current receive and send traffic in **Mbit/s or MB/s**, automatic Ethernet/Wi-Fi selection or a chosen adapter, and an adjustable ring scale. This is adapter traffic, including local-network transfers; it is not an Internet speed test.
- Independent switches for load, temperature, memory, drives and network.
- Vertical and horizontal layouts, automatic wrapping when space is limited, adjustable circle size, spacing and opacity.
- Four themes: dark, light, neon and monochrome.
- Monitor selection, edge and corner anchors, free dragging and optional always-on-top placement.
- A lock mode that passes mouse clicks through the widgets, plus a notification-area menu and global keyboard shortcuts.
- Optional startup at sign-in, including an administrator mode managed through Windows Task Scheduler.
- Hover details in a separate window that fits within the monitor work area, including when the circles sit vertically at the right edge.

A new profile places the widgets vertically at the **right-center of a secondary monitor**, when one is connected. Existing placement and appearance settings are preserved when updating.

## Getting started

1. Run the installer, or extract the complete portable ZIP and open `OrbitWidgets.exe`.
2. Right-click a circle to open **Настройки** (Settings). Choose the metrics, layout, theme and monitor you want.
3. Drag a circle to move the group freely, or select an anchor in Settings.
4. Enable **Поверх других окон** (Always on top) if you want the widgets visible over ordinary windows on that monitor.

| Action | Control |
| --- | --- |
| Open Settings and allow interaction | `Ctrl` + `Alt` + `O` |
| Toggle the lock / click-through mode | `Ctrl` + `Alt` + `L` |
| Open Settings | Right-click any circle, or use the notification-area icon |
| Exit | Notification-area menu → **Выход** |

CPU/GPU load, memory and network are sampled every **500 ms**. The sensor helper and drive read/write speeds update **once a second**; drive space updates every five seconds. Drive speeds come from Windows LogicalDisk performance counters. An unavailable speed appears as **—**, while an idle drive can report zero. Ring motion is smoothed independently of sampling. RAM and drive capacities use binary gigabytes (1024³ bytes), displayed as **ГБ**; drive and network speeds use decimal megabytes of 1,000,000 bytes, and 1 MB/s equals 8 Mbit/s.

In Settings, enable **Диски по буквам** and choose **Объём** or **Скорость**. Speed mode uses **↓** for reading and **↑** for writing; hover over a circle for capacity and both speeds. Existing profiles retain Capacity mode and their selected drive visibility until you change them.

## Sensor availability

The GPU circle can also show used / total dedicated video memory. Turn the line
off with **ГП · видеопамять** in Settings. Memory is read from the same NVIDIA
adapter once per second using NVML v2 with a v1 fallback; driver reserve accounting
can differ from Windows. Shared system RAM is excluded.

GPU load prefers **Windows WDDM** counters for the same uniquely identified NVIDIA adapter, using the busiest engine as Task Manager does. GPU temperature uses **NVML** from the installed NVIDIA driver without launching `nvidia-smi` for each sample. NVML is also a labelled load fallback if Windows counters are unavailable or the adapter cannot be mapped safely. The tooltip identifies the active load source; the two providers use different definitions and sample windows, so fallback readings can differ. When direct readings are unavailable, a bundled helper tries the CPU and GPU sensors exposed by **LibreHardwareMonitor 0.9.6**. Available readings depend on hardware, drivers and access permissions.

CPU temperature can also be read from the fresh shared-memory reading of an already running **MSI Afterburner**. Orbit Widgets does not install or start Afterburner. Direct CPU access through LibreHardwareMonitor may require the **PawnIO** driver and administrator rights.

The recommended installer configuration includes the official signed **PawnIO 2.2.0** installer and skips driver installation when a suitable version is already present. A configuration without driver installation is available. Driver installation requires Windows administrator approval; the application does not disable Windows security settings or force a restart. Elevated startup alone does not install the driver or guarantee a supported temperature sensor.

An unavailable sensor is shown as **—**, rather than a fabricated zero. Other metrics remain usable when a temperature sensor is missing. Exact third-party sources, hashes and notices are listed in [SENSOR-SOURCES.txt](build-assets/SENSOR-SOURCES.txt).

## Startup, settings and updates

In Settings, **Запуск → Запускать при входе в Windows** enables startup at sign-in. **С правами администратора** requests elevated startup. Windows may ask for administrator confirmation when this setting changes. The task runs in the current user's interactive session after a ten-second delay; no password is stored. Enabling startup does not change the privileges of the already running instance.

Settings are stored locally at `%LOCALAPPDATA%\Orbit Widgets\settings.json`, with a last-good copy in `settings.json.bak`. Changes are saved automatically; a damaged primary file is recovered from the backup. The application does not send monitoring readings to an Internet service. Portable and installed copies use this same current-user profile by default.

To update, close Orbit Widgets and install the new version, or replace the complete portable application folder with the new package. Keep your settings file. If a portable copy uses startup, keep its path stable or disable startup in the old copy and enable it again from the new location. The installer preserves the settings and does not remove the shared PawnIO driver when uninstalling the application.

## Technology and development

| Area | Technology |
| --- | --- |
| Main application | Rust, egui/eframe with an OpenGL renderer, native Win32 desktop integration |
| System metrics | Windows APIs, sysinfo, NVIDIA NVML |
| Sensor fallback | Small C# helper, .NET Framework, LibreHardwareMonitor |
| Packaging | PowerShell, Inno Setup 7.1, portable ZIP and SHA-256 checksums |

See [build instructions](docs/BUILD.md), [changes](CHANGELOG.md), [contribution guidance](CONTRIBUTING.md) and the [gallery renderer](docs/GALLERY.md). The main application uses a native desktop rendering stack and does not embed a web browser.

[GitHub Actions](https://github.com/feogrim-cloud/OrbitWidgets/actions) builds and checks Windows x64 packages. Version tags (`v*`) or a manual run with `release_tag` publish the installer and portable package only after the build and package checks succeed. Download `SHA256SUMS.txt` from the same release to verify the assets; the automated checks do not replace hardware sensor and desktop testing.

The project's own code is licensed under [MIT](LICENSE). Bundled third-party components retain their own licenses; their texts and source references accompany the packages. See [Rust notices](build-assets/THIRD-PARTY-RUST.txt) and [sensor component sources](build-assets/SENSOR-SOURCES.txt).
