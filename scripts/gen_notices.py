# 生成 docs/THIRD-PARTY-NOTICES.md 与 release/THIRD-PARTY-NOTICES.md
# 用法：python scripts/gen_notices.py（依赖 cargo tree 与 package-lock.json）
import io, json, re, subprocess
from collections import OrderedDict

out = subprocess.run(
    ["cargo", "tree", "--format", "{p}|{l}", "--prefix", "none", "-e", "normal"],
    cwd=r"D:\code\SkillDock\src-tauri", capture_output=True, text=True, encoding="utf-8",
).stdout
rust = OrderedDict()
for line in out.splitlines():
    line = line.strip()
    if not line or "|" not in line:
        continue
    pkg, lic = line.rsplit("|", 1)
    lic = lic.replace(" (*)", "")
    m = re.match(r"(.+?) v([0-9][^\s]*)", pkg.replace(" (proc-macro)", ""))
    if m:
        rust["{} {}".format(m.group(1), m.group(2))] = lic
rust_items = sorted(rust.items())

lock = json.load(open(r"D:\code\SkillDock\package-lock.json", encoding="utf-8"))
npm = OrderedDict()
for d, meta in (lock.get("packages") or {}).items():
    if not d.startswith("node_modules/") or d.count("node_modules/") > 1:
        continue
    name = d.replace("node_modules/", "")
    npm["{} {}".format(name, meta.get("version", ""))] = (meta.get("license") or "(未标注)").replace(" (*)", "")
npm_items = sorted(npm.items())

def family(lic):
    for f in ["Apache-2.0", "BSD-3-Clause", "BSD-2-Clause", "BSL-1.0", "CC-BY-4.0", "CC0-1.0",
              "ISC", "MIT-0", "MIT", "MPL-2.0", "Unicode-3.0", "Unlicense", "Zlib", "0BSD"]:
        if f in lic:
            return f
    return lic or "(未标注)"

fams = OrderedDict()
for _, lic in rust_items + npm_items:
    f = family(lic)
    fams[f] = fams.get(f, 0) + 1

def table(items):
    rows = ["| 包（名称 版本） | 许可证 |", "| --- | --- |"]
    rows += ["| `{}` | {} |".format(name, lic) for name, lic in items]
    return "\n".join(rows)

MIT = """MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."""

ISC = """ISC License

Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE."""

BSD3 = """BSD 3-Clause License

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE."""

UNLICENSE = """The Unlicense

This is free and unencumbered software released into the public domain.
Anyone is free to copy, modify, publish, use, compile, sell, or distribute this software, either in source code form or as a compiled binary, for any purpose, commercial or non-commercial, and by any means.
In jurisdictions that recognize copyright laws, the author or authors of this software dedicate any and all copyright interest in the software to the public domain.
THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. Full text: https://unlicense.org/"""

UNICODE30 = """UNICODE LICENSE V3

Permission is hereby granted, free of charge, to any person obtaining a copy of data files and any associated documentation or computer programs, to deal without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, and/or sell copies, provided that (a) this copyright and permission notice appear with all copies, (b) the license is available at https://www.unicode.org/license.txt, and (c) modified files carry prominent notices stating the modifications."""

ZLIB = """zlib License

This software is provided 'as-is', without any express or implied warranty. In no event will the authors be held liable for any damages arising from the use of this software. Permission is granted to anyone to use this software for any purpose, including commercial applications, and to alter it and redistribute it freely, subject to the following restrictions: 1. The origin of this software must not be misrepresented; 2. Altered source versions must be plainly marked as such; 3. This notice may not be removed or altered from any source distribution."""

BSL = """Boost Software License - Version 1.0

Permission is hereby granted, free of charge, to any person or organization obtaining a copy of the software and accompanying documentation covered by this license (the "Software") to use, reproduce, display, distribute, execute, and transmit the Software, and to prepare derivative works of the Software, and to permit third-parties to whom the Software is furnished to do so, all subject to the following: The copyright notices in the Software and this entire statement must be included in all copies of the Software, in whole or in part. THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND. Full text: https://www.boost.org/LICENSE_1_0.txt"""

ZERO_BSD = """0BSD License

Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted. THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES. Full text: https://opensource.org/license/0bsd"""

MIT0 = """MIT No Attribution (MIT-0)

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software. THE SOFTWARE IS PROVIDED "AS IS". Full text: https://opensource.org/license/mit-0"""

CC0 = """CC0 1.0 Universal

The person who associated a work with this deed has dedicated the work to the public domain by waiving all of his or her rights to the work worldwide under copyright law, including all related and neighboring rights, to the extent allowed by law. Full text: https://creativecommons.org/publicdomain/zero/1.0/legalcode"""

APACHE = """Apache License, Version 2.0

Full canonical text: https://www.apache.org/licenses/LICENSE-2.0.txt

Key terms: permission to use, reproduce, prepare derivative works, publicly display/perform, sublicense and distribute, subject to (4.1) including a copy of this License, (4.2) marking modified files, (4.3) retaining notices, (4.4) including NOTICE file contents. Includes express patent grant (section 3) and trademark limitation (section 6)."""

MPL = """Mozilla Public License, v. 2.0

Full canonical text: https://www.mozilla.org/MPL/2.0/

File-level weak copyleft: Covered Software files must remain under MPL-2.0 with source made available; may be combined into a Larger Work under different terms as long as MPL-covered files stay under MPL-2.0 (sections 3.1/3.3)."""

CCBY = """Creative Commons Attribution 4.0 International (CC-BY-4.0)

Applies to: caniuse-lite (browser support data, npm). Requirement: attribution to https://caniuse.com with a link to the license. Full text: https://creativecommons.org/licenses/by/4.0/legalcode"""

fam_rows = "\n".join("| {} | {} |".format(k, v) for k, v in sorted(fams.items(), key=lambda x: -x[1]))

doc = """# SkillDock 第三方依赖许可说明（Third-Party Notices）

生成日期：2026-09-09 · 生成方式：`python scripts/gen_notices.py`（Rust 经由 `cargo tree`；npm 经由 `package-lock.json`）· 票据：[#15](https://github.com/mrye111/SkillDock/issues/15)

本应用以静态链接方式分发 Rust 依赖、以打包方式分发前端依赖；未发现 GPL/LGPL/AGPL 类强 copyleft 依赖。

## 许可分布概览（去重后按包计数）

| 许可族 | 包数 |
| --- | --- |
{fam_rows}

## Rust 依赖清单

{rust_table}

## npm 依赖清单

{npm_table}

## 许可证文本与要点

### MIT License
```
{MIT}
```

### Apache License 2.0
{APACHE}

### ISC License
```
{ISC}
```

### BSD 3-Clause License
```
{BSD3}
```

### Mozilla Public License 2.0（适用于 cssparser/selectors/dtoa-short/option-ext 等）
{MPL}

### Unicode License V3（适用于 icu_*、litemap、tinystr、zerotrie 等 ICU4X 组件）
{UNICODE30}

### The Unlicense
{UNLICENSE}

### zlib License
{ZLIB}

### Boost Software License 1.0（适用于 ryu）
{BSL}

### 0BSD License
{ZERO_BSD}

### MIT No Attribution（MIT-0）
{MIT0}

### CC0 1.0 Universal
{CC0}

### CC-BY-4.0
{CCBY}

## Microsoft WebView2 Runtime

本应用运行依赖 **Microsoft Edge WebView2 Runtime**（专有组件，非本应用代码）：
- 标准安装包以 downloadBootstrapper 方式在缺失时引导下载（https://go.microsoft.com/fwlink/?linkid=2124701）；离线安装包内嵌微软官方离线安装组件。
- WebView2 按其自身许可条款分发：**Microsoft Software License Terms: Microsoft Edge WebView2 Runtime**（随安装组件展示；参考 https://learn.microsoft.com/microsoft-edge/webview2/）。
""".format(
    fam_rows=fam_rows,
    rust_table=table(rust_items),
    npm_table=table(npm_items),
    MIT=MIT, APACHE=APACHE, ISC=ISC, BSD3=BSD3, MPL=MPL, UNICODE30=UNICODE30,
    UNLICENSE=UNLICENSE, ZLIB=ZLIB, BSL=BSL, ZERO_BSD=ZERO_BSD, MIT0=MIT0, CC0=CC0, CCBY=CCBY,
)

for path in [r"D:\code\SkillDock\docs\THIRD-PARTY-NOTICES.md", r"D:\code\SkillDock\release\THIRD-PARTY-NOTICES.md"]:
    io.open(path, "w", encoding="utf-8", newline="\n").write(doc)
print("notices generated:", len(rust_items), "rust +", len(npm_items), "npm")
