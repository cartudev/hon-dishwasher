@echo off
rem Opens the dishwasher widget window.
setlocal
cd /d "%~dp0"
if not exist .venv\Scripts\python.exe (
  echo Missing the Python environment. Run setup.bat ^(or: python -m venv .venv ^& .venv\Scripts\pip install -r requirements.txt^)
  exit /b 1
)
set HON_DISHWASHER_PYTHON=%cd%\.venv\Scripts\python.exe
cargo run --release -- %*
