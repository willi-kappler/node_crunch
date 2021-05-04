all: 00_overview.png 01_start.png 02_compute.png 03_finish.png 04_heartbeat.png

.PHONY: clean

clean:
	rm *.aux *.fdb_latexmk *.fls *.dvi *.log *.ps *.pdf *.synctex.gz

00_overview.png: 00_overview.tex
	latex -shell-escape 00_overview.tex
	gm convert 00_overview.png -resize "10%" 00_overview_small.png
	rm 00_overview.png
	mv 00_overview_small.png 00_overview.png

01_start.png: 01_start.tex
	latex -shell-escape 01_start.tex
	gm convert 01_start.png -resize "10%" 01_start_small.png
	rm 01_start.png
	mv 01_start_small.png 01_start.png

02_compute.png: 02_compute.tex
	latex -shell-escape 02_compute.tex
	gm convert 02_compute.png -resize "10%" 02_compute_small.png
	rm 02_compute.png
	mv 02_compute_small.png 02_compute.png

03_finish.png: 03_finish.tex
	latex -shell-escape 03_finish.tex
	gm convert 03_finish.png -resize "10%" 03_finish_small.png
	rm 03_finish.png
	mv 03_finish_small.png 03_finish.png

04_heartbeat.png: 04_heartbeat.tex
	latex -shell-escape 04_heartbeat.tex
	gm convert 04_heartbeat.png -resize "10%" 04_heartbeat_small.png
	rm 04_heartbeat.png
	mv 04_heartbeat_small.png 04_heartbeat.png
