@echo off
diskpart /s "%~dp0PREP.TXT"
if not exist S:\ exit /b 1
if not exist T:\ exit /b 1
mkdir S:\Users\Fixture\Documents
mkdir T:\ArchRescue
echo wave4 protected source data>S:\Users\Fixture\Documents\fixture.txt
echo independent spare>S:\readme.txt
diskpart /s "%~dp0LOCK.TXT"
type S:\Users\Fixture\Documents\fixture.txt
