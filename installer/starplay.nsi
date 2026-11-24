; StarPlay installer (NSIS 3, Modern UI 2).
; Per-user install, no administrator rights: %LOCALAPPDATA%\Programs\StarPlay.
; Registers StarPlay as a video player (Open with, Settings > Default apps); Windows lets the user pick the default.
; Build: scripts/build-installer.ps1 (passes /DVERSION and /DSRC).

Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor /SOLID lzma

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef SRC
  !error "SRC (folder with starplay.exe, libmpv-2.dll, glass.glsl) is required"
!endif

!define APP "StarPlay"
!define EXE "starplay.exe"
!define PROGID "StarPlay.Video"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\StarPlay"
!define CAPS_KEY "Software\StarPlay\Capabilities"

Name "${APP}"
OutFile "..\target\StarPlay-${VERSION}-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\StarPlay"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
BrandingText "${APP} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "FileDescription" "${APP} installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "GPL-3.0-or-later"

!include "MUI2.nsh"

!define MUI_ICON "..\logo\StarPlay.ico"
!define MUI_UNICON "..\logo\StarPlay.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch StarPlay"
; Second checkbox: open Windows default apps settings to choose StarPlay as the video player
!define MUI_FINISHPAGE_SHOWREADME ""
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Choose StarPlay as my default video player"
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION OpenDefaultApps

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; Video extensions StarPlay offers to open
!macro ForEachExt MACRO
  !insertmacro ${MACRO} "mov"
  !insertmacro ${MACRO} "mp4"
  !insertmacro ${MACRO} "m4v"
  !insertmacro ${MACRO} "mkv"
  !insertmacro ${MACRO} "webm"
  !insertmacro ${MACRO} "avi"
  !insertmacro ${MACRO} "mxf"
  !insertmacro ${MACRO} "ts"
  !insertmacro ${MACRO} "m2ts"
  !insertmacro ${MACRO} "mts"
  !insertmacro ${MACRO} "wmv"
  !insertmacro ${MACRO} "flv"
  !insertmacro ${MACRO} "mpg"
  !insertmacro ${MACRO} "mpeg"
  !insertmacro ${MACRO} "ogv"
  !insertmacro ${MACRO} "3gp"
!macroend

!macro RegisterExt EXT
  WriteRegStr HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}" ""
  WriteRegStr HKCU "${CAPS_KEY}\FileAssociations" ".${EXT}" "${PROGID}"
  WriteRegStr HKCU "Software\Classes\Applications\${EXE}\SupportedTypes" ".${EXT}" ""
!macroend

!macro UnregisterExt EXT
  DeleteRegValue HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}"
!macroend

Function OpenDefaultApps
  ExecShell "open" "ms-settings:defaultapps?registeredAppUser=StarPlay"
FunctionEnd

Section "StarPlay" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "${SRC}\${EXE}"
  File "${SRC}\libmpv-2.dll"
  File "${SRC}\glass.glsl"
  File "..\LICENSE"
  File "..\CREDITS.md"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0

  ; Apps & features entry
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "URLInfoAbout" "https://github.com/adriendelmas/StarPlay"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" 140000

  ; File type (ProgID) and application registration
  WriteRegStr HKCU "Software\Classes\${PROGID}" "" "Video (StarPlay)"
  WriteRegStr HKCU "Software\Classes\${PROGID}" "FriendlyTypeName" "Video"
  WriteRegStr HKCU "Software\Classes\${PROGID}\DefaultIcon" "" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "Software\Classes\${PROGID}\shell\open\command" "" '"$INSTDIR\${EXE}" "%1"'
  WriteRegStr HKCU "Software\Classes\Applications\${EXE}" "FriendlyAppName" "${APP}"
  WriteRegStr HKCU "Software\Classes\Applications\${EXE}\DefaultIcon" "" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "Software\Classes\Applications\${EXE}\shell\open\command" "" '"$INSTDIR\${EXE}" "%1"'

  ; Capabilities: makes StarPlay appear in Settings > Default apps
  WriteRegStr HKCU "${CAPS_KEY}" "ApplicationName" "${APP}"
  WriteRegStr HKCU "${CAPS_KEY}" "ApplicationDescription" "Video player with frame-accurate checking tools"
  WriteRegStr HKCU "${CAPS_KEY}" "ApplicationIcon" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "Software\RegisteredApplications" "${APP}" "${CAPS_KEY}"
  !insertmacro ForEachExt RegisterExt

  ; Tell Explorer that file associations changed
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\libmpv-2.dll"
  Delete "$INSTDIR\glass.glsl"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\CREDITS.md"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP}.lnk"

  !insertmacro ForEachExt UnregisterExt
  DeleteRegValue HKCU "Software\RegisteredApplications" "${APP}"
  DeleteRegKey HKCU "Software\StarPlay"
  DeleteRegKey HKCU "Software\Classes\${PROGID}"
  DeleteRegKey HKCU "Software\Classes\Applications\${EXE}"
  DeleteRegKey HKCU "${UNINST_KEY}"
  ; Settings (recent files) are kept in %APPDATA%\StarPlay

  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
