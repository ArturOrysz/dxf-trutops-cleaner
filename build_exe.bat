@echo off
setlocal

cd /d "%~dp0"

echo [1/3] Instalacja zaleznosci...
python -m pip install -r requirements.txt
if errorlevel 1 goto :fail

echo [2/3] Budowanie EXE...
python -m PyInstaller --noconfirm --clean --onefile --windowed --name dxf_trutops_cleaner --hidden-import arc_fit --hidden-import simplify_dxf --exclude-module matplotlib --exclude-module PIL --exclude-module pillow dxf_gui.py
if errorlevel 1 goto :fail

echo [3/3] Gotowe.
echo EXE: %~dp0dist\dxf_trutops_cleaner.exe
exit /b 0

:fail
echo Budowanie nie powiodlo sie.
exit /b 1
