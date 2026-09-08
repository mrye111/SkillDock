#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Seedance 2.0 知识库自动更新脚本
自动抓取三个官方参考文档并更新知识库
"""

import os
import json
import re
from datetime import datetime
from pathlib import Path

# 配置
SKILL_DIR = Path(__file__).parent.parent
REFERENCES_DIR = SKILL_DIR / "references"
UPDATES_FILE = REFERENCES_DIR / "latest_updates.md"
LOG_FILE = REFERENCES_DIR / "update_log.md"

# 官方参考文档 URL
OFFICIAL_DOCS = [
    {
        "name": "Seedance 2.0 科幻场景生成实战手册",
        "url": "https://m.blog.csdn.net/u014177256/article/details/158035544",
        "id": "csdn_scifi"
    },
    {
        "name": "Seedance 2.0 提示词完全指南",
        "url": "https://m.blog.csdn.net/misslelover/article/details/158040401",
        "id": "csdn_guide"
    },
    {
        "name": "小云雀产品手册",
        "url": "https://bytedance.larkoffice.com/docx/OJ3jdZ6e2oWvIuxfb1Kczsk8nVe",
        "id": "lark_manual"
    }
]

def fetch_url_content(url: str) -> str:
    """使用 web_fetch 工具抓取 URL 内容"""
    try:
        from urllib.request import urlopen, Request
        from urllib.error import URLError

        headers = {
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
        }
        req = Request(url, headers=headers)
        with urlopen(req, timeout=30) as response:
            charset = response.headers.get_content_charset() or 'utf-8'
            content = response.read().decode(charset)
            return content
    except Exception as e:
        return f"<!-- Error fetching {url}: {e} -->"

def extract_markdown_content(html: str) -> str:
    """从 HTML 中提取 Markdown 格式内容"""
    # 移除 script 和 style 标签
    html = re.sub(r'<script[^>]*>.*?</script>', '', html, flags=re.DOTALL | re.IGNORECASE)
    html = re.sub(r'<style[^>]*>.*?</style>', '', html, flags=re.DOTALL | re.IGNORECASE)

    # 转换基本 HTML 标签为 Markdown
    replacements = [
        (r'<h1[^>]*>(.*?)</h1>', r'# \1\n'),
        (r'<h2[^>]*>(.*?)</h2>', r'## \1\n'),
        (r'<h3[^>]*>(.*?)</h3>', r'### \1\n'),
        (r'<h4[^>]*>(.*?)</h4>', r'#### \1\n'),
        (r'<p[^>]*>(.*?)</p>', r'\1\n\n'),
        (r'<br\s*/?>', r'\n'),
        (r'<strong[^>]*>(.*?)</strong>', r'**\1**'),
        (r'<b[^>]*>(.*?)</b>', r'**\1**'),
        (r'<em[^>]*>(.*?)</em>', r'*\1*'),
        (r'<i[^>]*>(.*?)</i>', r'*\1*'),
        (r'<li[^>]*>(.*?)</li>', r'- \1\n'),
        (r'<code[^>]*>(.*?)</code>', r'`\1`'),
        (r'<a[^>]*href=["\']([^"\']+)["\'][^>]*>(.*?)</a>', r'[\2](\1)'),
    ]

    for pattern, replacement in replacements:
        html = re.sub(pattern, replacement, html, flags=re.DOTALL | re.IGNORECASE)

    # 移除剩余 HTML 标签
    html = re.sub(r'<[^>]+>', '', html)

    # 清理多余空白
    html = re.sub(r'\n{3,}', r'\n\n', html)
    html = html.strip()

    return html

def extract_useful_content(markdown: str) -> str:
    """从 Markdown 中提取有用内容（过滤导航、页脚等）"""
    lines = markdown.split('\n')
    useful_lines = []
    skip_patterns = [
        r'^#', r'^##',  # 标题可能有用
        r'登录', r'注册', r'首页', r'博客', r'论坛',
        r'©.*CSDN', r'收藏', r'点赞', r'评论',
        r'^$', r'^\s+$'
    ]

    for line in lines:
        line = line.strip()
        if not line:
            continue
        skip = False
        for pattern in skip_patterns:
            if re.match(pattern, line):
                skip = True
                break
        if not skip:
            useful_lines.append(line)

    return '\n'.join(useful_lines)

def generate_update_content(doc: dict, content: str) -> str:
    """生成更新内容块"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    return f"""
## {doc['name']}

**更新时间**: {timestamp}
**来源**: {doc['url']}

### 核心内容

{content}

---
"""

def update_knowledge_base():
    """主更新函数"""
    print("🚀 开始更新 Seedance 2.0 知识库...")

    # 确保目录存在
    REFERENCES_DIR.mkdir(parents=True, exist_ok=True)

    all_updates = []
    update_summary = []

    for doc in OFFICIAL_DOCS:
        print(f"📥 抓取: {doc['name']}...")
        try:
            html = fetch_url_content(doc['url'])
            if html and not html.startswith("<!-- Error"):
                markdown = extract_markdown_content(html)
                useful = extract_useful_content(markdown)
                all_updates.append(generate_update_content(doc, useful))
                update_summary.append(f"✅ {doc['name']} - 成功")
            else:
                update_summary.append(f"⚠️ {doc['name']} - 抓取失败")
        except Exception as e:
            update_summary.append(f"❌ {doc['name']} - 异常: {e}")

    # 写入更新文件
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    updates_content = f"""# Seedance 2.0 最新文档内容

> 自动更新于 {timestamp}
> 通过 `scripts/update_knowledge.py` 脚本抓取官方文档

---

""".strip() + "\n\n".join(all_updates)

    UPDATES_FILE.write_text(updates_content, encoding='utf-8')
    print(f"💾 已保存到: {UPDATES_FILE}")

    # 更新日志
    log_entry = f"""
## {timestamp}

**更新摘要**:
""" + "\n".join(update_summary)

    if LOG_FILE.exists():
        existing_log = LOG_FILE.read_text(encoding='utf-8')
        log_entry = existing_log + "\n" + log_entry
    else:
        log_entry = f"# 更新日志\n\n{log_entry}"

    LOG_FILE.write_text(log_entry, encoding='utf-8')
    print(f"📝 已更新日志: {LOG_FILE}")

    # 输出摘要
    print("\n📊 更新摘要:")
    for item in update_summary:
        print(f"   {item}")

    print("\n✨ 更新完成!")
    return update_summary

if __name__ == "__main__":
    update_knowledge_base()
