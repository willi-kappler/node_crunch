all: 00_overview.png

.PHONY: clean

clean:
	rm *.aux *.dvi *.log *.ps

00_overview.png: 00_overview.tex
	latex -shell-escape 00_overview.tex
