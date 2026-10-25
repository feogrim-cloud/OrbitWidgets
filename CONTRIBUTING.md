# Contributing to Orbit Widgets

Bug reports and focused pull requests are welcome. Discuss a substantial feature in an [issue](https://github.com/feogrim-cloud/OrbitWidgets/issues) before changing several parts of the application.

## Reporting a problem

Include the Orbit Widgets version, Windows version, steps to reproduce, expected behavior and actual behavior. For placement or tooltip problems, include the number of monitors, their arrangement, scaling, selected monitor and widget orientation. For missing sensors, include the CPU/GPU model, driver version and the sensor status shown in Settings. A missing reading is not the same as a zero reading.

Remove personal paths and other private information from screenshots and logs before posting them. Do not upload your settings profile or access tokens. Never attach passwords or administrator credentials.

## Making a change

- Keep changes compatible with Windows x64 and the supported Windows versions described in the README.
- Preserve existing settings, monitor placement and current-user startup behavior. A new default must not overwrite an existing choice.
- Keep unavailable or stale readings visibly unavailable. Avoid fabricated zero temperatures and avoid summing physical and virtual network adapters into duplicate traffic.
- Keep sampling outside the UI thread and preserve passive window behavior, click-through locking and the global unlock shortcut.
- Keep driver installation a separate installer component. Do not disable Windows security settings, force a restart or silently broaden application privileges.
- Preserve third-party license texts, source references and checksum checks when changing bundled components.
- Do not commit build products, local toolchains, profiles, logs, diagnostics or temporary download/inspection folders.

See [Build from source](docs/BUILD.md). Run the checks appropriate to the change; the normal application build runs the Rust tests. A UI change also needs a manual check in the affected layout and monitor arrangement. For sensor changes, describe which real hardware and data sources were tested. CI cannot prove hardware sensor availability.

Pull requests should explain the resulting behavior and the checks performed. Keep one pull request focused on one task. The project uses [MIT](LICENSE) for its own code; bundled dependencies retain their own licenses.
