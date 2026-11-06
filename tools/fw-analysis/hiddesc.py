import sys
d=open(sys.argv[1],'rb').read()
start=int(sys.argv[2],16); maxlen=int(sys.argv[3]) if len(sys.argv)>3 else 400
TAGS={(0,8):'Input',(0,9):'Output',(0,11):'Feature',(0,10):'Collection',(0,12):'EndCollection',
(1,0):'UsagePage',(1,1):'LogMin',(1,2):'LogMax',(1,3):'PhysMin',(1,4):'PhysMax',(1,7):'ReportSize',(1,8):'ReportID',(1,9):'ReportCount',
(2,0):'Usage',(2,1):'UsageMin',(2,2):'UsageMax'}
i=start; depth=0; end=start+maxlen
while i<end:
    b=d[i]; size=[0,1,2,4][b&3]; typ=(b>>2)&3; tag=b>>4
    name=TAGS.get((typ,tag))
    if name is None: print(f"-- stop at {hex(i)} byte {b:02x}"); break
    v=int.from_bytes(d[i+1:i+1+size],'little',signed=False)
    if name=='EndCollection': depth-=1
    print("  "*depth+f"{name} 0x{v:x}" + (f" ({v})" if name in('ReportSize','ReportCount','ReportID','LogMax','LogMin') else ''))
    if name=='Collection': depth+=1
    i+=1+size
    if name=='EndCollection' and depth==0:
        # continue if next looks like a usage page
        if i<len(d) and d[i] in (0x05,0x06): continue
        print(f"-- end at {hex(i)}"); break
