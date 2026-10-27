# Interface images

The `gpu-memory` scene renders the actual GPU circle at 83 and 128 px with fixed
dedicated-memory readings. It uses the same isolated, no-profile renderer.

The `disk-speed` scene renders Capacity and Speed views side by side at 83 and
128 px on a 620 × 240 canvas. The values are demonstration readings; both views
use the application's actual disk gauge code. It can also be exported separately:

```powershell
cargo run --release --locked --features gallery --example gallery -- --scene disk-speed --output assets/screenshots/disk-speed.png
```

The `ram-memory` scene renders the RAM circle at 83 and 128 px with occupied
percentage, used memory, total capacity and available physical memory in the
same circle. Export it separately:

```powershell
cargo run --release --locked --features gallery --example gallery -- --scene ram-memory --output assets/screenshots/ram-memory.png
```

The PNG gallery is exported from the application's actual egui gauge and settings
rendering code. Readings are fixed demonstration values. The development renderer
does not read or save a user profile, collect hardware metrics, register hotkeys,
create startup tasks, install drivers or control an existing application instance.

Run from a Windows development environment with Cargo and a native linker:

```powershell
.\scripts\Render-Screenshots.ps1
```

The script builds the optional `gallery` example and saves seven PNGs under
`assets/screenshots`. A custom folder can be passed with `-OutputDir`.
The renderer uses eframe's own screenshot export and exits after capture.

The regular application build does not enable the `gallery` feature. The renderer
uses the installed Windows Segoe UI font, as does the main application; that font
is read locally and is not redistributed. Neutral backgrounds in the pictures
provide contrast for transparent widgets and are not part of the desktop overlay.
