Orbit Widgets Windows installer

Compiler: official Inno Setup 7.1.0 x64, portable extraction.
Download: https://github.com/jrsoftware/issrc/releases/download/is-7_1_0/innosetup-7.1.0-x64.exe
Compiler setup SHA256: 0362A383ED217D4C4239B5933866DD96D3EB2102737DA92F80F6057A4B40DF2F
Authenticode checked Valid, signer Pyrsys B.V.
Portable mode is documented at https://jrsoftware.org/ishelp/topic_technotes.htm
No system PATH, file associations, shortcuts, uninstaller, or Add/Remove Programs entry was requested for the portable compiler.

Build the production installer after updating dist with the final app build:
  powershell -NoProfile -File installer\build-installer.ps1
The helper verifies the exact official PawnIO 2.2.0 SHA256 and its Authenticode signature before compilation.
Default compiler: ..\toolchain\inno\ISCC.exe relative to the project work directory.
Default output: outputs\OrbitWidgets-Setup-x64.exe in this chat's workspace.

Direct compiler equivalent:
  ISCC.exe /DSourceDir="ABSOLUTE\dist" /DBuildAssetsDir="ABSOLUTE\build-assets" /DOutputDir="ABSOLUTE\outputs" installer\OrbitWidgets.iss

Production behavior:
  - 64-bit x64 installer; Windows 10 2004 or newer.
  - Program Files\OrbitWidgets, Start Menu shortcuts, optional desktop shortcut.
  - Recommended component selection includes the official signed PawnIO 2.2.0 driver for CPU temperatures.
  - Existing PawnIO DisplayVersion >= 2.2.0 is preserved without reinstallation.
  - Embedded prerequisite is checked against its known SHA256 before execution.
  - Driver command is -install -silent, default signed edition only.
  - Prerequisite exit 0 requires confirming its registered version. Exit 3010 continues with restart-required status. Every other exit stops Setup with an error.
  - Interactive setup offers a restart choice when necessary. Silent mode never restarts Windows automatically and returns 3010 instead.
  - For deployments, use /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG="path" and handle exit 0/3010 separately from failures.
  - Normal/elevated autostart is configured in the program's settings, not silently enabled by Setup.
  - Uninstall calls OrbitWidgets.exe --startup-configure disable and checks the result before removing files.
  - Uninstall preserves user settings in LocalAppData and the shared PawnIO driver.

Non-administrative QA build and install:
  powershell -NoProfile -File installer\build-installer.ps1 -QA
  OrbitWidgets-Setup-QA.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /DIR="ABSOLUTE\work\installer-qa-install" /QAREPORT="ABSOLUTE\work\installer-qa-report.txt" /LOG="ABSOLUTE\work\installer-qa.log"
QA is a separate compile-time mode, not an option that changes production behavior.
It requests lowest privileges, copies the real payload into the chosen directory, never executes PawnIO, creates no icons/uninstaller/registry entries, changes no autostart, and does not launch the app.
Its initialization self-checks numeric version comparison (including 2.10 vs 2.2), invalid versions, success, reboot-required, and failure exit-code classification.
Compare the installed OrbitWidgets.exe/sensor/driver hashes with staged dist and build-assets after the QA copy.
No production installer or driver was executed during package preparation.
