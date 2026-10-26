; Compile with Inno Setup 7.1+.
; SourceDir, BuildAssetsDir, OutputDir and AppVersion can be overridden with /D.
; OrbitSetupQA builds a non-elevating, file-copy-only test installer.
#ifndef AppVersion
  #define AppVersion "1.1.3"
#endif
#ifndef SourceDir
  #define SourceDir AddBackslash(SourcePath) + "..\dist"
#endif
#ifndef BuildAssetsDir
  #define BuildAssetsDir AddBackslash(SourcePath) + "..\build-assets"
#endif
#ifndef OutputDir
  #ifdef OrbitSetupQA
    #define OutputDir AddBackslash(SourcePath) + "..\..\installer-qa-build"
  #else
    #define OutputDir AddBackslash(SourcePath) + "..\..\..\outputs"
  #endif
#endif
#define PawnIOHash "1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032"

[Setup]
#ifdef OrbitSetupQA
AppId=OrbitWidgets-QA-5DC99268-9889-4C87-BED2-EC35655BC55E
AppName=Orbit Widgets — проверка установщика
PrivilegesRequired=lowest
Uninstallable=no
CreateUninstallRegKey=no
DefaultDirName={src}\OrbitWidgets-QA
OutputBaseFilename=OrbitWidgets-Setup-QA
#else
AppId={{5DC99268-9889-4C87-BED2-EC35655BC55E}
AppName=Orbit Widgets
PrivilegesRequired=admin
Uninstallable=yes
CreateUninstallRegKey=yes
DefaultDirName={autopf}\OrbitWidgets
OutputBaseFilename=OrbitWidgets-Setup-x64
AppMutex=Local\OrbitWidgetsInstance
#endif
AppVersion={#AppVersion}
AppVerName=Orbit Widgets {#AppVersion}
AppPublisher=Orbit Widgets
DefaultGroupName=Orbit Widgets
SetupArchitecture=x64
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.19041
OutputDir={#OutputDir}
SetupIconFile={#BuildAssetsDir}\orbit.ico
UninstallDisplayIcon={app}\orbit.ico
VersionInfoVersion={#AppVersion}.0
VersionInfoDescription=Orbit Widgets Setup
WizardStyle=modern
DisableProgramGroupPage=yes
DisableWelcomePage=no
DisableReadyPage=no
AllowNoIcons=yes
Compression=lzma2/max
SolidCompression=yes
CloseApplications=yes
CloseApplicationsFilter=OrbitWidgets.exe,OrbitSensors.exe
RestartApplications=no
AlwaysRestart=no
RestartIfNeededByRun=no
SetupLogging=yes
ShowLanguageDialog=no
UsePreviousAppDir=yes
UsePreviousGroup=yes
UsePreviousTasks=yes

[Languages]
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Types]
Name: "full"; Description: "Рекомендуемая установка"
#ifndef OrbitSetupQA
Name: "compact"; Description: "Программа без установки датчика ЦП"
Name: "custom"; Description: "Выбор компонентов"; Flags: iscustom
#endif

[Components]
#ifdef OrbitSetupQA
Name: "main"; Description: "Файлы программы — проверка без изменения системы"; Types: full; Flags: fixed
#else
Name: "main"; Description: "Orbit Widgets"; Types: full compact custom; Flags: fixed
Name: "pawnio"; Description: "Датчик температуры ЦП — подписанный PawnIO 2.2.0"; Types: full custom
#endif

#ifndef OrbitSetupQA
[Tasks]
Name: "desktopicon"; Description: "Создать ярлык на рабочем столе"; GroupDescription: "Ярлыки:"; Flags: checkedonce
#endif

[Files]
; The optional driver installer is listed once so it can also be extracted
; before file installation by PrepareToInstall. It remains available to the app.
Source: "{#SourceDir}\*"; DestDir: "{app}"; Excludes: "optional\PawnIO-Setup.exe,.source\*,*.log"; Flags: ignoreversion recursesubdirs createallsubdirs; Components: main
Source: "{#BuildAssetsDir}\optional\PawnIO-Setup.exe"; DestDir: "{app}\optional"; Flags: ignoreversion; Components: main

#ifndef OrbitSetupQA
[Icons]
Name: "{group}\Orbit Widgets"; Filename: "{app}\OrbitWidgets.exe"; WorkingDir: "{app}"; IconFilename: "{app}\orbit.ico"
Name: "{group}\Настройки Orbit Widgets"; Filename: "{app}\OrbitWidgets.exe"; Parameters: "--settings"; WorkingDir: "{app}"; IconFilename: "{app}\orbit.ico"
Name: "{autodesktop}\Orbit Widgets"; Filename: "{app}\OrbitWidgets.exe"; WorkingDir: "{app}"; IconFilename: "{app}\orbit.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\OrbitWidgets.exe"; Description: "Запустить Orbit Widgets"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent runasoriginaluser
#endif

[Code]
const
  PawnIOUninstallKey = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO';
  ExpectedPawnIOHash = '{#PawnIOHash}';

var
  DriverRebootRequired: Boolean;
  DriverPrerequisiteComplete: Boolean;

function NextVersionPart(var Value: String): Integer;
var
  Separator: Integer;
  Part: String;
begin
  if Value = '' then begin
    Result := 0;
    Exit;
  end;
  Separator := Pos('.', Value);
  if Separator = 0 then begin
    Part := Value;
    Value := '';
  end else begin
    Part := Copy(Value, 1, Separator - 1);
    Delete(Value, 1, Separator);
  end;
  Result := StrToIntDef(Trim(Part), -1);
end;

function VersionAtLeast(Value, Minimum: String): Boolean;
var
  Index, CurrentPart, RequiredPart: Integer;
begin
  Result := False;
  for Index := 0 to 3 do begin
    CurrentPart := NextVersionPart(Value);
    RequiredPart := NextVersionPart(Minimum);
    if (CurrentPart < 0) or (RequiredPart < 0) then Exit;
    if CurrentPart > RequiredPart then begin
      Result := True;
      Exit;
    end;
    if CurrentPart < RequiredPart then Exit;
  end;
  Result := True;
end;

function PawnIOIsCurrent: Boolean;
var
  Version: String;
begin
  Result := False;
  if RegQueryStringValue(HKLM64, PawnIOUninstallKey, 'DisplayVersion', Version) then begin
    Log('PawnIO version (64-bit registry): ' + Version);
    if VersionAtLeast(Version, '2.2.0.0') then begin
      Result := True;
      Exit;
    end;
  end;
  if RegQueryStringValue(HKLM32, PawnIOUninstallKey, 'DisplayVersion', Version) then begin
    Log('PawnIO version (32-bit registry): ' + Version);
    Result := VersionAtLeast(Version, '2.2.0.0');
  end;
end;

function DriverExitClass(const ExitCode: Integer): Integer;
begin
  case ExitCode of
    0: Result := 0;
    3010: Result := 1;
  else
    Result := -1;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
#ifndef OrbitSetupQA
var
  InstallerPath, ActualHash: String;
  ExitCode: Integer;
#endif
begin
  Result := '';
#ifdef OrbitSetupQA
  Log('OrbitSetupQA: driver execution is disabled at compile time.');
#else
  if not WizardIsComponentSelected('pawnio') then begin
    Log('PawnIO component was not selected. No driver changes requested.');
    Exit;
  end;
  if DriverPrerequisiteComplete then Exit;
  if PawnIOIsCurrent then begin
    Log('PawnIO 2.2.0 or newer already installed. Skipping prerequisite.');
    DriverPrerequisiteComplete := True;
    Exit;
  end;
  try
    ExtractTemporaryFile('PawnIO-Setup.exe');
    InstallerPath := ExpandConstant('{tmp}\PawnIO-Setup.exe');
    ActualHash := GetSHA256OfFile(InstallerPath);
    if CompareText(ActualHash, ExpectedPawnIOHash) <> 0 then begin
      Result := 'Не удалось проверить целостность модуля датчика ЦП. Установка остановлена.';
      Exit;
    end;
    Log('Verified signed official PawnIO 2.2.0 payload SHA-256: ' + ActualHash);
    if not Exec(InstallerPath, '-install -silent', ExpandConstant('{tmp}'),
      SW_HIDE, ewWaitUntilTerminated, ExitCode) then begin
      Result := 'Не удалось запустить установку датчика ЦП: ' + SysErrorMessage(ExitCode);
      Exit;
    end;
    Log('PawnIO installer exit code: ' + IntToStr(ExitCode));
    case DriverExitClass(ExitCode) of
      0: begin
        if not PawnIOIsCurrent then begin
          Result := 'Установщик датчика ЦП завершился, но установленная версия PawnIO 2.2.0 не подтверждена. Установка остановлена.';
          Exit;
        end;
        DriverPrerequisiteComplete := True;
      end;
      1: begin
        DriverRebootRequired := True;
        DriverPrerequisiteComplete := True;
        Log('PawnIO requires a restart. Setup will not restart Windows automatically in silent mode.');
      end;
    else
      Result := 'Не удалось установить датчик температуры ЦП (код ' + IntToStr(ExitCode) + '). Установка остановлена.';
    end;
  except
    Result := 'Не удалось подготовить датчик температуры ЦП: ' + GetExceptionMessage;
  end;
#endif
end;

function NeedRestart: Boolean;
begin
  // Silent/very-silent setup must never cause an automatic restart. Signal
  // success-with-reboot-required through its exit code instead.
  Result := DriverRebootRequired and not WizardSilent;
end;

function GetCustomSetupExitCode: Integer;
begin
  if DriverRebootRequired then Result := 3010 else Result := 0;
end;

#ifdef OrbitSetupQA
function InitializeSetup: Boolean;
var
  ReportPath, ReportText: String;
  LogicPassed: Boolean;
begin
  LogicPassed := VersionAtLeast('2.2.0', '2.2.0.0')
    and VersionAtLeast('2.10.0.0', '2.2.0.0')
    and not VersionAtLeast('2.1.0.0', '2.2.0.0')
    and not VersionAtLeast('invalid', '2.2.0.0')
    and (DriverExitClass(0) = 0)
    and (DriverExitClass(3010) = 1)
    and (DriverExitClass(1603) = -1);
  if not LogicPassed then RaiseException('Installer prerequisite logic self-check failed.');
  ReportPath := ExpandConstant('{param:QAREPORT|}');
  if ReportPath <> '' then begin
    ReportText := 'OrbitSetupQA=1' + #13#10 + 'DriverExecution=disabled' + #13#10
      + 'VersionAndExitCodeChecks=passed' + #13#10
      + 'RegistryWrites=disabled' + #13#10 + 'Icons=disabled' + #13#10
      + 'AutostartChanges=disabled' + #13#10 + 'PostInstallLaunch=disabled' + #13#10;
    if not SaveStringToFile(ReportPath, ReportText, False) then
      RaiseException('Cannot write QA report.');
  end;
  Result := True;
end;
#else
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppPath: String;
  ExitCode: Integer;
begin
  if CurUninstallStep <> usUninstall then Exit;
  AppPath := ExpandConstant('{app}\OrbitWidgets.exe');
  if FileExists(AppPath) then begin
    if not Exec(AppPath, '--startup-configure disable', ExpandConstant('{app}'),
      SW_HIDE, ewWaitUntilTerminated, ExitCode) then
      RaiseException('Не удалось отключить автозапуск Orbit Widgets: ' + SysErrorMessage(ExitCode));
    if ExitCode <> 0 then
      RaiseException('Не удалось подтвердить отключение автозапуска Orbit Widgets (код ' + IntToStr(ExitCode) + '). Файлы программы не удалены.');
    Log('Orbit Widgets current-user autostart disabled and verified.');
  end;
  // Shared PawnIO and user settings deliberately remain installed.
end;
#endif
