# Build Orbit Widgets on Windows

Orbit Widgets currently targets **Windows x64**. The Rust application uses Win32 APIs; the sensor helper and installer are Windows components. These instructions do not produce Linux, x86 or ARM64 packages.

## Prerequisites

- Rust and Cargo for an x64 Windows target, with a working native linker.
- Either the **MSVC** Rust toolchain with Visual Studio C++ Build Tools and the Windows SDK, or the **GNU** Rust toolchain with a complete x64 MinGW-w64 toolchain such as w64devkit on `PATH`.
- Windows PowerShell or PowerShell 7.
- The .NET Framework C# compiler at `%WINDIR%\Microsoft.NET\Framework64\v4.0.30319\csc.exe`. The helper targets x64 and loads the supplied .NET Framework sensor assemblies; a separate .NET SDK is not required for this helper.
- **Inno Setup 7.1 or newer** to build the EXE installer. The application and portable files can be built without Inno Setup.

Use a Rust toolchain whose host target matches the linker you installed. `rustup show active-toolchain` and `rustc -vV` show the active configuration. The repository's `.cargo/config.toml` adds self-contained linking for `x86_64-pc-windows-gnu`; it does not change the target of an MSVC toolchain.

Pinned third-party runtime files under `build-assets/sensors` and the optional driver installer under `build-assets/optional` are build inputs. Their exact origins, SHA-256 hashes and licenses are recorded in [SENSOR-SOURCES.txt](../build-assets/SENSOR-SOURCES.txt). Keep the notices alongside these files and do not substitute unrelated DLLs. `build-assets/sensors/.source` is temporary inspection data and is not required.

## Test and build

From the repository root:

```powershell
.\build.ps1
```

The script runs `cargo test --release --locked`, builds the Rust executable, compiles `sensor-helper/Program.cs`, and stages the application and runtime files under `dist`.

```powershell
.\dist\OrbitWidgets.exe
```

Keep the staged `sensors` folder beside `OrbitWidgets.exe`. NVIDIA NVML is loaded from the installed NVIDIA driver and is not redistributed with Orbit Widgets.

To run the Rust checks separately:

```powershell
cargo test --release --locked
```

`build.ps1 -SkipTests` is available when the exact source has already passed the required checks. Keep `Cargo.lock` committed and use locked dependencies for release builds. Generated files in `target` and `dist` do not belong in source control.

## Build the installer

After a successful application build, pass the location of your Inno compiler explicitly. This avoids relying on a local workspace toolchain directory:

```powershell
.\installer\build-installer.ps1 `
  -SourceDir .\dist `
  -Compiler 'C:\Tools\InnoSetup7\ISCC.exe' `
  -OutputDir .\release
```

The result is `release/OrbitWidgets-Setup-x64.exe`. Adjust the example compiler path to your installation. The script checks the pinned PawnIO installer hash and its Authenticode publisher before compiling the package. Building the installer does not install or execute the driver.

The version in `Cargo.toml`, `Cargo.lock`, `README-RU.txt`, the installer definition and the installer script defaults must agree for a release. `-AppVersion` can explicitly select the installer version; do not use it to label a different application build.

## Installer verification without a driver installation

The `-QA` switch builds a separate file-copy-only test installer:

```powershell
.\installer\build-installer.ps1 `
  -SourceDir .\dist `
  -Compiler 'C:\Tools\InnoSetup7\ISCC.exe' `
  -OutputDir .\release-qa `
  -QA
```

This produces `OrbitWidgets-Setup-QA.exe`. The QA build disables driver execution, application registration, shortcuts, startup changes and the post-install application launch. Use it in a disposable test directory to verify the staged payload. It is a test artifact, not the public installer. A successful QA copy does not prove that a real CPU sensor or elevated startup works on every computer.

## Release checks

- Run the Rust tests and compile a fresh application payload.
- Confirm the installer and portable archive contain the same intended x64 application build and all required sensor runtime files and notices.
- Verify archive integrity and publish SHA-256 checksums for the download assets.
- Check the affected UI manually, including a narrow vertical layout, the edge of a secondary monitor, negative desktop coordinates where applicable, and differing display scales.
- For startup changes, verify task ownership and privilege mode in the actual interactive Windows session, then clean up temporary test tasks.
- For sensor changes, record the hardware and real provider tested. Test missing and stale readings as well as successful readings; do not install a kernel driver merely to run a generic build check.

Release tags use `v` followed by the application version, for example `v1.1.1`. Keep a portable application in a permanent location before enabling startup. Personal profiles, diagnostics, toolchain downloads and temporary test artifacts must not be included in a source archive or release package.

## Packaging and continuous integration

After building the application and public installer, use the repository packaging script to create the portable archive and source archive and write `SHA256SUMS.txt`:

```powershell
python .\scripts\package.py --output-dir release
```

Run the release verification script against those files and a separately built QA installer:

```powershell
.\scripts\Test-Release.ps1 `
  -ReleaseDir .\release `
  -QAInstaller .\.build-qa\OrbitWidgets-Setup-QA.exe
```

Build the QA installer with `-OutputDir .\.build-qa -QA` when using this example. The public installer and the QA installer are distinct outputs. Do not upload the QA installer as an end-user release asset.

[GitHub Actions](https://github.com/feogrim-cloud/OrbitWidgets/actions) performs the Windows x64 build with Rust 1.99.1 and the MSVC toolchain. It verifies the official Inno Setup 7.1 download and publisher, runs the Rust checks, builds and tests the packages, and saves build artifacts. Publication is triggered by a version tag (`v*`) or a manual `workflow_dispatch` run with `release_tag`, and waits for the build job to pass. Release notes are maintained in [RELEASE_NOTES.md](RELEASE_NOTES.md); downloaded assets can be checked against the release's `SHA256SUMS.txt`.
