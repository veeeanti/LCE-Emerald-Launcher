import zlib

def decode_varint(data, offset=0):
    val = 0
    shift = 0
    while True:
        b = data[offset]
        offset += 1
        val |= (b & 0x7F) << shift
        shift += 7
        if (b & 0x80) == 0:
            break
    return val, offset

hex_str = "81 9b 01 78 9c ed 5b cd 92 db 44 10 9e 14 f9 71 9c 4a 41 48"
data = bytes.fromhex(hex_str.replace(" ", ""))

uncompressed_len, offset = decode_varint(data)
compressed_data = data[offset:]

d = zlib.decompressobj()
dec = d.decompress(compressed_data)
if dec:
    pkt_id, dec_offset = decode_varint(dec)
    print(f"Packet ID: {pkt_id} (0x{pkt_id:02x})")
    print(f"Next bytes: {dec[dec_offset:dec_offset+10].hex()}")
else:
    print("Could not decompress anything (need more bytes).")
