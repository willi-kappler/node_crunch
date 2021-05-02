all: 00_overview.png 01_start.png 02_compute.png

.PHONY: clean

clean:
	rm *.aux *.dvi *.log *.ps

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
