import sys,struct,re,collections
d=open(sys.argv[1],'rb').read()
# find the "key_press" behavior device name and other behavior names
for n in [b'key_press',b'kp\x00',b'mkp',b'key_toggle',b'momentary_layer',b'layer_tap',b'transparent',b'keychron',b'user_custom',b'bluetooth',b'outputs',b'mouse_key_press']:
    for m in re.finditer(re.escape(n),d): pass
# ZMK binding = {char* dev, u32 p1, u32 p2}; find runs of 12-byte records where dev pointers repeat
recs=collections.Counter()
cons=collections.Counter(); kbd=collections.Counter()
for off in range(0,len(d)-12,4):
    p,a,b=struct.unpack_from('<III',d,off)
    page=(a>>16)&0xff; uid=a&0xffff; mods=a>>24
    if page==0x0c and 0<uid<0x400 and b==0 and (p&0xff000000) in (0x00000000,0x20000000,0x00800000,0x08000000) and p>0x1000:
        cons[(uid,p)]+=1
    if page==0x07 and 0<uid<0xE8 and b==0 and p>0x1000:
        kbd[p]+=1
# the behaviour pointer most used for keyboard page = key_press device
kp=[p for p,c in kbd.most_common(3)]
print("likely key_press dev ptrs:",[hex(x) for x in kp], [kbd[x] for x in kp])
seen=collections.Counter()
for (uid,p),c in cons.items():
    if p in kp: seen[uid]+=c
for uid,c in sorted(seen.items()): print("consumer 0x%03x x%d"%(uid,c))
# also keyboard page codes > 0x65 used (F13+, intl, etc.)
k2=collections.Counter()
for off in range(0,len(d)-12,4):
    p,a,b=struct.unpack_from('<III',d,off)
    if p in kp and ((a>>16)&0xff)==7 and b==0: k2[(a&0xffff, a>>24)]+=1
print("keyboard-page usages (usage, implicit-mods):")
print(sorted((hex(u),hex(m),c) for (u,m),c in k2.items() if u>=0x46 or m))
