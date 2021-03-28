@echo This might overwrite existing area and socials.txt files, and you will lose any changes.
@echo Are you sure you want to continue?
@pause

git clone https://github.com/mudhistoricalsociety/dawnoftime_1.69r
git clone https://github.com/DikuMUDOmnibus/Ultra-Envy

xcopy /s dawnoftime_1.69r\area area\
xcopy Ultra-Envy\sys\SOCIALS.TXT socials.txt
