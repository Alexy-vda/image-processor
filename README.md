# image-processor

CLI tool to copy CR2/MP4 files from an SD card to a destination folder, organized by shooting session.

Files are grouped into sessions based on a configurable time gap (default: 6 hours) between consecutive files. Each session gets its own dated folder.

## Installation

```bash
cargo build --release
sudo cp target/release/image-processor /usr/local/bin/
```

## Usage

```bash
image-processor --input /Volumes/EOS_DIGITAL --output ~/Photos
```

### Options

| Flag | Description | Default |
|---|---|---|
| `-i, --input` | Input directory (SD card, folder with CR2/MP4 files) | required |
| `-o, --output` | Output directory where session folders are created | required |
| `--gap-hours` | Minimum gap in hours to split into a new session | `6` |
| `--dry-run` | Preview session grouping without copying files | `false` |

### Examples

Preview what would happen without copying:

```bash
image-processor -i /Volumes/EOS_DIGITAL -o ~/Photos --dry-run
```

Use a 3-hour gap to split sessions:

```bash
image-processor -i /Volumes/EOS_DIGITAL -o ~/Photos --gap-hours 3
```

## How it works

1. **Scan** the input directory recursively for `.CR2` and `.MP4` files
2. **Extract** the sequence number from each filename (e.g. `_MG_1001.CR2` -> `1001`)
3. **Sort** files by sequence number
4. **Read metadata** (EXIF for CR2, mvhd for MP4, filesystem date as fallback)
5. **Group** into sessions: a new session starts when the time gap between two consecutive files exceeds the threshold
6. **Name** session folders by date (`2024-01-15`), with a suffix when multiple sessions fall on the same day (`2024-01-15_a`, `2024-01-15_b`)
7. **Copy** files with a progress bar, saving state after each file for resume support

## Resume support

If a transfer is interrupted (Ctrl+C, crash, etc.), re-running the same command will skip already copied files and continue where it left off. A `.image-processor-state.json` file tracks progress and is automatically cleaned up after a successful transfer.

## Output structure

```
~/Photos/
  2024-01-15/
    _MG_1001.CR2
    _MG_1002.CR2
    MVI_1003.MP4
  2024-01-16_a/
    _MG_1050.CR2
    _MG_1051.CR2
  2024-01-16_b/
    _MG_1080.CR2
    _MG_1081.CR2
```
