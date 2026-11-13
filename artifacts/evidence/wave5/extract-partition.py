import json, sys, struct
from pathlib import Path
src, dst = map(Path, sys.argv[1:3])
partition_number = int(sys.argv[3])
with src.open('rb') as f, dst.open('xb') as out:
    header = f.read(440)
    map_offset, data_offset = struct.unpack_from('<II', header, 340)
    size, block, overhead, count = struct.unpack_from('<QIII', header, 368)
    assert block == 1024**2 and not overhead
    f.seek(map_offset)
    blocks = struct.unpack(f'<{count}I', f.read(count*4))
    def at(offset, length):
        index, inner = divmod(offset, block)
        assert inner + length <= block
        if blocks[index] >= 0xfffffffe:
            return bytes(length)
        f.seek(data_offset + blocks[index]*block + inner)
        return f.read(length)
    disk = at(512, 512)
    lba, entries, entry_size = struct.unpack_from('<QII', disk, 72)
    assert disk[:8] == b'EFI PART' and 0 < partition_number <= entries and entry_size == 128
    record = at(lba*512 + (partition_number-1)*entry_size, entry_size)
    first, last = struct.unpack_from('<QQ', record, 32)
    assert 0 < first <= last < size//512
    start, end = first*512, (last+1)*512
    out.truncate(end-start)
    while start < end:
        index, inner = divmod(start, block)
        length = min(block-inner, end-start)
        if blocks[index] < 0xfffffffe:
            out.write(at(start, length))
        else:
            out.seek(length, 1)
        start += length
