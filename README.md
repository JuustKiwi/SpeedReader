# SpeedReader

This a speed reading tool that uses the Rapid Serial Visual Presentation technique with highlighting of the Optimal Recognition Point, this allows the user to read at much higher WPM than with normal reading

This tool works only with .pdf , .txt and .docx files

## Requirements

Ensure you have the following installed on your system:
* Rust and Cargo
* GCC / G++ compiler
* Make
* Poppler and Poppler-CPP development headers

## Build and Install

```bash
make install
```

## Usagee

If you have a pdf file you can read by page or by chapter, reading by chapter is more complicated since it works by extracting embedded PDF metadata, not all PDFs have the chapters stored in their metada so it doesn't always work.

To read by page do:
```bash
speedreader <path_to_pdf> -p <start_page> <end_page>
```

To read by chapter do:
```bash
speedreader <path_to_pdf> -c <chapter_number>
```

To read a txt file or a docx file:

```bash
speedreader <path_to_file>
```


### In-App Controls
* **Space**: Play / Pause
* **Up / Down Arrows**: Increase or decrease reading speed
* **Left / Right Arrows**: Go backward and forward through the text
* **Q or Esc**: Quit the application

## Configuration

You can customize your default WPM and the terminal UI colors by creating a configuration file at `~/.config/speedreader.conf`.

Example `speedreader.conf`:
```ini
wpm = 450
highlight_color = cyan
text_color = white
```

