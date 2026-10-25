# Interface images

The PNG gallery is exported from the application's actual egui gauge and settings
rendering code. Readings are fixed demonstration values. The development renderer
does not read or save a user profile, collect hardware metrics, register hotkeys,
create startup tasks, install drivers or control an existing application instance.

Run from a Windows development environment with Cargo and a native linker:

```powershell
.\scripts\Render-Screenshots.ps1
```

The script builds the optional `gallery` example and saves four PNGs under
`assets/screenshots`. A custom folder can be passed with `-OutputDir`.
The renderer uses eframe's own screenshot export and exits after capture.

The regular application build does not enable the `gallery` feature. The renderer
uses the installed Windows Segoe UI font, as does the main application; that font
is read locally and is not redistributed. Neutral backgrounds in the pictures
provide contrast for transparent widgets and are not part of the desktop overlay.
