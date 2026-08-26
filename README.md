# SpeedReader

This a speed reading tool that uses the Rapid Serial Visual Presentation technique with highlighting of the Optimal Recognition Point, this allows the user to read at much higher WPM than with normal reading

This tool works only with .pdf , .txt , .epub and .docx files

This tool also has it's own .sr file type, this file type allows you to add up to 50 chapters in a book. It also remembers where you stopped reading so the next time you read it will just pickup where you stopped. Note that this also tracks when you are reading in --view mode, so if you just want to scroll through the book make sure to either add a chapter to remember where you stopped or to write it down somewhere.

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

Read Standard:   ```bash speedreader <file> [-w <word_idx> | -p <virtual_page> | --view]```
Read PDF:        ```bash speedreader <file.pdf> [-c <chapter> | -p <start_page> <end_page> | --view]```
Read SR Binary:  ```bash speedreader <file.sr> [-c <chapter> | -w <word_idx> | -p <virtual_page> | --view]```
Compile Binary:  ```bash speedreader <file> --compile <out.sr>```
Manage Chapters: ```bash speedreader <file.sr> --add-chapter <word_idx> "<Title>"```
List Chapters:   ```bash speedreader <file.sr> --list-chapters```
View Stats:      ```bash speedreader <file> --stats```
Help: 			 ```bash speedreader -h```

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

