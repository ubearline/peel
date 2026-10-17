; peel NSIS installer — install exe + register right-click shell command
!define APPNAME "peel"
!define COMPANY "peel"
!define VERSION "0.1.0"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\peel"

Name "${APPNAME} ${VERSION}"
OutFile "peel-setup.exe"
InstallDir "$PROGRAMFILES64\peel"
RequestExecutionLevel admin
SetCompressor lzma

Page directory
Page instfiles
UninstPage instfiles

; Register right-click entries under HKCU\Software\Classes (merged into HKCR view).
; Macros so install and uninstall can never drift apart.
!macro REG_PEEL
  WriteRegStr HKCU "Software\Classes\*\shell\peel" "MUIVerb" "Extract with peel"
  WriteRegStr HKCU "Software\Classes\*\shell\peel" "Icon" "$INSTDIR\peel.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\peel\command" "" '"$INSTDIR\peel.exe" "%1"'
  ; Also on .zip and .7z ProgIDs so it shows even where * is shadowed
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel" "MUIVerb" "Extract with peel"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel" "Icon" "$INSTDIR\peel.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel\command" "" '"$INSTDIR\peel.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel" "MUIVerb" "Extract with peel"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel" "Icon" "$INSTDIR\peel.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel\command" "" '"$INSTDIR\peel.exe" "%1"'
!macroend

!macro UNREG_PEEL
  DeleteRegKey HKCU "Software\Classes\*\shell\peel"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel"
!macroend

Section "Install"
  SetOutPath $INSTDIR
  File "target\x86_64-pc-windows-gnu\release\peel.exe"
  !insertmacro REG_PEEL

  ; Uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "peel"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${COMPANY}"
SectionEnd

Section "Uninstall"
  !insertmacro UNREG_PEEL
  DeleteRegKey HKLM "${UNINST_KEY}"
  Delete "$INSTDIR\peel.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
