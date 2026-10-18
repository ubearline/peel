; peel NSIS installer (MUI2) — modern wizard, drive-letter adaptive install dir
Unicode true
!define APPNAME "peel"
!define COMPANY "peel"
; VERSION is injected: makensis -DVERSION=x.y.z installer.nsi (CI passes the git tag)
!ifndef VERSION
  !define VERSION "0.0.0-dev"
!endif
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\peel"

!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "${APPNAME} ${VERSION}"
OutFile "peel-setup.exe"
RequestExecutionLevel admin
SetCompressor lzma

; ---- adaptive default install dir: 2nd fixed drive -> X:\app\peel, else Program Files ----
Var DriveCount
Var SecondDrive

Function .onInit
  StrCpy $DriveCount 0
  StrCpy $SecondDrive ""
  ${GetDrives} "HDD" GetDrivesCallback
  ${If} $SecondDrive != ""
    StrCpy $INSTDIR "$SecondDriveapp\peel"
  ${Else}
    StrCpy $INSTDIR "$PROGRAMFILES64\peel"
  ${EndIf}
FunctionEnd

; $9 = drive path (e.g. "D:\") for each fixed drive found
Function GetDrivesCallback
  IntOp $DriveCount $DriveCount + 1
  ${If} $DriveCount = 2
    StrCpy $SecondDrive $9
    Push "StopGetDrives"
  ${Else}
    Push "KeepSearching"
  ${EndIf}
FunctionEnd

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"

; Register right-click entry under HKCU\Software\Classes (merged into HKCR view).
; Only .zip / .7z — peel only supports those formats.
; Single entry, always extracts into a NEW numbered folder; never touches existing files.
!macro REG_PEEL
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel" "MUIVerb" "解压到同名文件夹"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel" "Icon" "$INSTDIR\peel.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel\command" "" '"$INSTDIR\peel.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel" "MUIVerb" "解压到同名文件夹"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel" "Icon" "$INSTDIR\peel.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel\command" "" '"$INSTDIR\peel.exe" "%1"'
!macroend

!macro UNREG_PEEL
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.zip\shell\peel"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.7z\shell\peel"
  ; cleanup from older versions that registered the * wildcard
  DeleteRegKey HKCU "Software\Classes\*\shell\peel"
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
