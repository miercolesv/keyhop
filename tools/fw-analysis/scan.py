import sys,re
d=open(sys.argv[1],'rb').read()
print("size",len(d))
# longer strings
for m in re.finditer(rb'[\x20-\x7e]{8,}',d):
    s=m.group().decode()
    if re.search(r'[A-Za-z]{4}',s): print(hex(m.start()),s[:100])
# UTF-16LE strings (USB string descriptors)
for m in re.finditer(rb'(?:[\x20-\x7e]\x00){4,}',d):
    print("u16",hex(m.start()),m.group().decode('utf-16le'))
# USB device descriptor: 12 01 xx xx .. .. .. 40/08 34 34 pid
for m in re.finditer(rb'\x12\x01..',d,re.S):
    i=m.start(); dd=d[i:i+18]
    if len(dd)==18 and dd[8:10]==b'\x34\x34':
        print("DEVDESC",hex(i),"VID 3434 PID %02x%02x"%(dd[11],dd[10]), "bcdDevice %02x%02x"%(dd[13],dd[12]))
# HID report descriptor starts
pats={b'\x05\x01\x09\x06':'GD Keyboard',b'\x05\x01\x09\x02':'GD Mouse',b'\x05\x0c\x09\x01':'Consumer Control',b'\x05\x01\x09\x80':'System Control',b'\x06\x60\xff':'Vendor FF60 (VIA raw HID)',b'\x06\x00\xff':'Vendor FF00',b'\x06\x31\xff':'Vendor FF31',b'\x06\x0c\xff':'Vendor FF0C',b'\x05\x01\x09\x06\xa1\x01':'kbd collection'}
for p,n in pats.items():
    for m in re.finditer(re.escape(p),d): print("HID",n,hex(m.start()))
