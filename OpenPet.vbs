' ==============================================================================
' OpenPet Silent Launcher (Zero Console Windows)
' Starts the OpenPet Companion seamlessly in the background.
' ==============================================================================
Option Explicit

Dim WshShell, FSO, strScriptDir, strHostExe, strControlExe, intResult

Set WshShell = CreateObject("WScript.Shell")
Set FSO = CreateObject("Scripting.FileSystemObject")

strScriptDir = FSO.GetParentFolderName(WScript.ScriptFullName)

' Check for release binary first, then fallback to debug
strHostExe = strScriptDir & "\target\release\openpet-host.exe"
If Not FSO.FileExists(strHostExe) Then
    strHostExe = strScriptDir & "\target\debug\openpet-host.exe"
End If

If FSO.FileExists(strHostExe) Then
    ' Set current directory to application root
    WshShell.CurrentDirectory = strScriptDir
    ' Run completely silent (0 = SW_HIDE, False = don't wait)
    WshShell.Run """" & strHostExe & """", 0, False
Else
    intResult = MsgBox("OpenPet derlenmemis gorunuyor. Once derleme calistirilsin mi?" & vbCrLf & vbCrLf & _
                       "OpenPet binaries were not found. Would you like to run the build launcher now?", _
                       vbYesNo + vbQuestion, "OpenPet")
    If intResult = vbYes Then
        WshShell.Run """" & strScriptDir & "\Run-OpenPet.bat""", 1, False
    End If
End If

Set WshShell = Nothing
Set FSO = Nothing
