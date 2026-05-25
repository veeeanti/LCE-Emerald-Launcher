import re

with open("ignore/MCProtocolLib/protocol/src/main/java/org/geysermc/mcprotocollib/protocol/codec/MinecraftCodec.java", "r") as f:
    lines = f.readlines()

in_game = False
clientbound_index = 0

for line in lines:
    if "ProtocolState.GAME" in line:
        in_game = True
        continue
    
    if in_game:
        # Check if another state starts
        if "ProtocolState." in line and "ProtocolState.GAME" not in line:
            break
            
        if "registerClientboundPacket" in line:
            # Extract the class name
            m = re.search(r'registerClientboundPacket\((.*?)\.class', line)
            if m:
                packet_class = m.group(1)
                print(f"{packet_class} -> {clientbound_index} (0x{clientbound_index:02x})")
                clientbound_index += 1
