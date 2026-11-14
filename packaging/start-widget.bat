@echo off
rem Starts the installed widget with its Python environment.
setlocal
cd /d "%~dp0"
set HON_DISHWASHER_PYTHON=%~dp0.venv\Scripts\python.exe
start "" "%~dp0hon-dishwasher.exe"
