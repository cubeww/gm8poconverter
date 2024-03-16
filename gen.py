import os 
for f in os.listdir('./gm8poconverter/src/gml/http'):
    name=f.split('.')[0]
    print(f'''
    assets.scripts.push(Some(Box::new(Script {{
            name: "{name}".into(),
            source: include_str!("./gml/http/{f}").into(),
        }})));
    ''')