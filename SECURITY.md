# Security behavior / Безопасность

Orbit Widgets reads local performance counters and sensors. It does not upload monitoring readings. Use the complete release package and keep the `sensors` directory beside the application; third-party components and their licenses are listed in the accompanying notices.

## Startup and installation

Elevated startup in 1.1.8 requires the application to be under the Program Files directory resolved by Windows. Before requesting elevation to configure startup or registering elevated startup, the application checks ownership and ACLs of the executable, bundled files and directories, and the parent chain that could replace them. Reparse points and installations writable by untrusted accounts are rejected. Use the installer and retain the normal Program Files permissions for this mode. Ordinary portable launches and non-elevated startup remain supported.

This restriction applies to the persistent startup task. The separate manual administrator restart still requests Windows UAC consent and does not create a task; for a portable copy, trust must cover its executable, helper and adjacent DLLs before consenting. No UAC bypass is claimed to have been found or fixed.

The startup task is registered through Windows Task Scheduler COM with its XML held in memory; no task XML is passed through a writable temporary file. Task ownership is checked before updates or deletion. The task uses the current user's interactive session and stores no password. Windows supplies the system, Windows and Program Files paths through native APIs rather than environment overrides.

## Sensors and fonts

Helper output is limited to 16 KiB per record. Invalid or oversized output clears its readings and triggers recovery; ten seconds without valid output triggers a restart. A Windows Job Object terminates the helper when the parent exits. A failed hardware refresh does not publish cached sensor values, and GPU values are selected from a single unambiguous adapter. NVML reconnects after a failure, with thirty seconds between automatic attempts; the sensor refresh control requests an immediate attempt.

The GUI uses egui/eframe 0.34.3 with the maintained skrifa/vello_cpu font stack, removing the former unmaintained ttf-parser dependency. The optional Windows UI font is read only from the OS-resolved Windows Fonts directory, with a 16 MiB size limit. No arbitrary font file is loaded from settings or an environment-provided path.

## Практические условия

Для повышенного автозапуска используйте установку в Program Files с обычными защищёнными правами. Проверяются владелец, права файлов и каталогов, а также отсутствие перенаправляющих ссылок. Переносная версия продолжает работать и поддерживает обычный автозапуск. Настройки текущего пользователя находятся отдельно от исполняемых файлов; схема профиля в 1.1.8 не меняется.

Задание автозапуска передаётся Планировщику через COM в памяти, без временного XML-файла. Поток модуля датчиков ограничен по размеру, зависший модуль восстанавливается, а при выходе приложения его завершает Windows Job Object. Ошибка датчика даёт недоступное значение, а не старое показание. Обновлённый шрифтовый движок не использует прежний неподдерживаемый ttf-parser.
