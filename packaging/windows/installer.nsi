Unicode true

!include "MUI2.nsh"

!ifndef VERSION
  !error "VERSION is required"
!endif
!ifndef SOURCE_DIR
  !error "SOURCE_DIR is required"
!endif
!ifndef NUMERIC_VERSION
  !error "NUMERIC_VERSION is required"
!endif
!ifndef OUTPUT_DIR
  !error "OUTPUT_DIR is required"
!endif

Name "Recentry ${VERSION}"
OutFile "${OUTPUT_DIR}\Recentry-${VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Recentry"
InstallDirRegKey HKCU "Software\Recentry" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${NUMERIC_VERSION}"
VIAddVersionKey /LANG=1033 "ProductName" "Recentry"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription" "Recent project launcher"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Copyright (c) 2026 sean2077"

!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\recentry.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "show"
!define MUI_FINISHPAGE_RUN_TEXT "Open Recentry"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${SOURCE_DIR}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

Function .onInit
  SetRegView 64
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

Function un.RemoveStartMenuShortcut
  StrCpy $0 20

shortcut_retry:
  ClearErrors
  Delete "$SMPROGRAMS\Recentry.lnk"
  IfErrors 0 shortcut_removed
  Sleep 100
  IntOp $0 $0 - 1
  IntCmp $0 0 shortcut_failed shortcut_retry shortcut_retry

shortcut_failed:
  DetailPrint "Failed to remove $SMPROGRAMS\Recentry.lnk"
  SetErrorLevel 1

shortcut_removed:
FunctionEnd

Section "Recentry" SEC_RECENTRY
  SetShellVarContext current
  SetOutPath "$INSTDIR"

  IfFileExists "$INSTDIR\recentry.exe" 0 +3
    ExecWait '"$INSTDIR\recentry.exe" quit'
    Sleep 250

  File "/oname=recentry.exe" "${SOURCE_DIR}\recentry.exe"
  File "/oname=recentry-ui.exe" "${SOURCE_DIR}\recentry-ui.exe"
  File "${SOURCE_DIR}\README.md"
  File "${SOURCE_DIR}\CHANGELOG.md"
  File "${SOURCE_DIR}\LICENSE"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\Recentry.lnk" "$INSTDIR\recentry.exe" "show"

  WriteRegStr HKCU "Software\Recentry" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "DisplayName" "Recentry"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "Publisher" "sean2077"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext current

  IfFileExists "$INSTDIR\recentry.exe" 0 +3
    ExecWait '"$INSTDIR\recentry.exe" quit'
    Sleep 250

  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Recentry"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry"
  DeleteRegKey HKCU "Software\Recentry"
  Call un.RemoveStartMenuShortcut

  Delete "$INSTDIR\recentry.exe"
  Delete "$INSTDIR\recentry-ui.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\CHANGELOG.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
