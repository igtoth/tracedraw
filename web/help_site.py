#!/usr/bin/env python3
"""Build the user Help (docs/help) into a static site, one tree per
interface language.

    python3 web/help_site.py [docs/help] [apps/tracedraw/lang] [site/help]

Pages are Markdown in docs/help/<lang>/<page>.md, ordered by
docs/help/contents.txt. A page missing in a language is published from the
English page with a notice, so every address exists in every language.
`[[menu.file > menu.file.save_as]]` is replaced by the interface labels of
that language's lang/<code>.toml, so the Help names menus and fields
exactly as the app does. `{#id}` after a heading gives it a stable anchor,
the same in every language. Standard library only; no network.

--partial skips listed pages not written yet; --lang=en,pt-BR builds only
those languages (both for checking work in progress).

Fails (exit 1) on a broken link, an unknown label key or a page that is
listed but missing in English. See docs/behavior/help.md.
"""

import html
import json
import os
import re
import shutil
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RTL = {"ar"}
ERRORS = []
PARTIAL = False
ONLY = set()
ALL_LISTED = set()


def err(msg):
    ERRORS.append(msg)


# ---------------------------------------------------------------- inputs


def languages():
    """(code, native name) in the app's order, from src/i18n.rs."""
    src = open(os.path.join(ROOT, "apps/tracedraw/src/i18n.rs"), encoding="utf-8").read()
    block = src[src.index("pub const LANGUAGES"):]
    block = block[: block.index("];")]
    return re.findall(r'\("([^"]+)",\s*"([^"]+)"\)', block)


def read_lang(path):
    """The flat `key = "value"` tables of lang/*.toml."""
    table = {}
    if not os.path.exists(path):
        return table
    for line in open(path, encoding="utf-8"):
        line = line.strip()
        if not line or line.startswith("#") or " = " not in line:
            continue
        key, _, rest = line.partition(" = ")
        rest = rest.strip()
        if not (rest.startswith('"') and rest.endswith('"')):
            continue
        out, i, body = [], 0, rest[1:-1]
        while i < len(body):
            c = body[i]
            if c == "\\" and i + 1 < len(body):
                n = body[i + 1]
                out.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\"}.get(n, n))
                i += 2
                continue
            out.append(c)
            i += 1
        table[key.strip()] = "".join(out)
    return table


def read_contents(path):
    """[(section page, [child pages])] from contents.txt."""
    tree = []
    for raw in open(path, encoding="utf-8"):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        page = raw.strip()
        if raw[0] in " \t":
            if not tree:
                err("contents.txt: an indented page before any section")
                continue
            tree[-1][1].append(page)
        else:
            tree.append((page, []))
    return tree


# -------------------------------------------------------------- markdown


def slug(text):
    s = re.sub(r"<[^>]+>", "", text).lower()
    s = re.sub(r"[^\w\s-]", "", s, flags=re.UNICODE)
    s = re.sub(r"[\s_-]+", "-", s).strip("-")
    return s or "section"


def label(table, en, key):
    v = table.get(key)
    if v is None:
        v = en.get(key)
    if v is None:
        return None
    v = v.split("\n")[0].strip()
    for tail in ("...", "…", ":", "："):
        if v.endswith(tail):
            v = v[: -len(tail)].rstrip()
    return v


CJK = re.compile(r"[\u3000-\u30ff\u3400-\u9fff\uf900-\ufaff\uff00-\uffef]")


def join_lines(parts):
    """Join source lines with a space, except between two CJK characters,
    where a line break is not a space."""
    out = ""
    for p in parts:
        p = p.strip()
        if out and p:
            out += "" if CJK.match(out[-1]) and CJK.match(p[0]) else " "
        out += p
    return out


class Page:
    def __init__(self, path, lang, md, table, en_table, fallback):
        self.path = path
        self.lang = lang
        self.fallback = fallback
        self.table = table
        self.en = en_table
        self.title = path
        self.headings = []  # (level, id, text)
        self.links = []  # (target page, anchor)
        self.text = []
        self.images = []
        self.body = self.render(md)

    # Inline: code, labels, links, bold, italics, line breaks.
    def inline(self, s):
        codes = []

        def keep(h):
            codes.append(h)
            return "\x00%d\x00" % (len(codes) - 1)

        s = re.sub(r"`([^`]+)`", lambda m: keep("<code>%s</code>" % html.escape(m.group(1))), s)

        def ui(m):
            parts = []
            for key in m.group(1).split(">"):
                key = key.strip()
                v = label(self.table, self.en, key)
                if v is None:
                    err("%s/%s: unknown label [[%s]]" % (self.lang, self.path, key))
                    v = key
                parts.append('<span class="ui">%s</span>' % html.escape(v))
            return keep('<span class="path">%s</span>' % '<span class="sep"> &gt; </span>'.join(parts))

        s = re.sub(r"\[\[([^\[\]]+)\]\]", ui, s)

        def image(m):
            alt, src = m.group(1), m.group(2)
            full = os.path.normpath(os.path.join(self.lang, os.path.dirname(self.path), src)).replace(os.sep, "/")
            if not full.startswith("img/"):
                err("%s/%s: image %s is not in docs/help/img" % (self.lang, self.path, src))
            self.images.append(full)
            return keep('<img src="%s" alt="%s" loading="lazy">' % (html.escape(src), esc(alt)))

        s = re.sub(r"!\[([^\]]*)\]\(([^)\s]+)\)", image, s)
        s = re.sub(r"<kbd>([^<]+)</kbd>", lambda m: keep("<kbd>%s</kbd>" % html.escape(m.group(1))), s)

        def finish(t):
            t = html.escape(t, quote=False)
            t = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", t)
            t = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", t)
            return re.sub(r"\x00(\d+)\x00", lambda m: codes[int(m.group(1))], t)

        def link(m):
            text, href = m.group(1), m.group(2)
            if re.match(r"^[a-z]+:", href):
                return keep('<a href="%s" rel="noopener">%s</a>' % (html.escape(href), finish(text)))
            target, _, anchor = href.partition("#")
            if target:
                if not target.endswith(".md"):
                    err("%s/%s: link %s is not a .md page" % (self.lang, self.path, href))
                full = os.path.normpath(os.path.join(os.path.dirname(self.path), target[:-3])).replace(os.sep, "/")
                self.links.append((full, anchor))
                out = target[:-3] + ".html" + ("#" + anchor if anchor else "")
            else:
                self.links.append((self.path, anchor))
                out = "#" + anchor
            return keep('<a href="%s">%s</a>' % (html.escape(out), finish(text)))

        s = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", link, s)
        return finish(s)

    def plain(self, s):
        return html.unescape(re.sub(r"<[^>]+>", "", self.inline(s)))

    def render(self, md):
        lines = md.replace("\r\n", "\n").split("\n")
        out = []
        i = 0
        para = []
        used_ids = set()

        def flush():
            if para:
                t = join_lines(para)
                inner = self.inline(t)
                if re.fullmatch(r"<img [^>]+>", inner):
                    out.append('<figure class="shot">%s</figure>' % inner)
                else:
                    out.append("<p>%s</p>" % inner)
                self.text.append(self.plain(t))
                para.clear()

        while i < len(lines):
            line = lines[i]
            st = line.strip()
            if not st:
                flush()
                i += 1
                continue
            if st.startswith("```"):
                flush()
                j = i + 1
                code = []
                while j < len(lines) and not lines[j].strip().startswith("```"):
                    code.append(lines[j])
                    j += 1
                out.append("<pre><code>%s</code></pre>" % html.escape("\n".join(code)))
                i = j + 1
                continue
            m = re.match(r"^(#{1,4})\s+(.*?)\s*(\{#([a-z0-9-]+)\})?\s*$", st)
            if m:
                flush()
                level = len(m.group(1))
                text = m.group(2)
                hid = m.group(4) or slug(self.plain(text))
                base, n = hid, 2
                while hid in used_ids:
                    hid = "%s-%d" % (base, n)
                    n += 1
                used_ids.add(hid)
                if level == 1:
                    self.title = self.plain(text)
                    out.append("<h1>%s</h1>" % self.inline(text))
                else:
                    self.headings.append((level, hid, self.plain(text)))
                    out.append(
                        '<h%d id="%s">%s<a class="anchor" href="#%s" aria-hidden="true">#</a></h%d>'
                        % (level, hid, self.inline(text), hid, level)
                    )
                self.text.append(self.plain(text))
                i += 1
                continue
            if st.startswith(">"):
                flush()
                quote = []
                while i < len(lines) and lines[i].strip().startswith(">"):
                    quote.append(lines[i].strip()[1:].strip())
                    i += 1
                t = join_lines(quote)
                cls = "note"
                if re.match(r"^\*\*[^*]+\*\*", t) and t.lower().startswith(("**tip", "**dica")):
                    cls = "tip"
                out.append('<div class="%s">%s</div>' % (cls, self.inline(t)))
                self.text.append(self.plain(t))
                continue
            if st.startswith("|") and i + 1 < len(lines) and re.match(r"^\|?\s*:?-+", lines[i + 1].strip()):
                flush()
                rows = []
                while i < len(lines) and lines[i].strip().startswith("|"):
                    rows.append([c.strip() for c in lines[i].strip().strip("|").split("|")])
                    i += 1
                head, body = rows[0], rows[2:]
                t = ["<div class=\"table\"><table><thead><tr>"]
                t += ["<th>%s</th>" % self.inline(c) for c in head]
                t.append("</tr></thead><tbody>")
                for r in body:
                    t.append("<tr>%s</tr>" % "".join("<td>%s</td>" % self.inline(c) for c in r))
                    self.text.append(" ".join(self.plain(c) for c in r))
                t.append("</tbody></table></div>")
                out.append("".join(t))
                continue
            if re.match(r"^(\s*)([-*]|\d+\.)\s+", line):
                flush()
                block = []
                while i < len(lines) and (
                    re.match(r"^(\s*)([-*]|\d+\.)\s+", lines[i])
                    or (lines[i].startswith("  ") and lines[i].strip() and block)
                ):
                    block.append(lines[i])
                    i += 1
                out.append(self.list_html(block))
                continue
            para.append(line)
            i += 1
        flush()
        return "\n".join(out)

    def list_html(self, block):
        """Nested lists by indentation; continuation lines join the item."""
        items = []  # (indent, ordered, text)
        for line in block:
            m = re.match(r"^(\s*)([-*]|\d+\.)\s+(.*)$", line)
            if m:
                items.append([len(m.group(1)), m.group(2)[0].isdigit(), m.group(3)])
            elif items:
                items[-1][2] = join_lines([items[-1][2], line])
        out = []
        stack = []  # (indent, tag)
        for indent, ordered, text in items:
            tag = "ol" if ordered else "ul"
            while stack and indent < stack[-1][0]:
                out.append("</li></%s>" % stack.pop()[1])
            if not stack or indent > stack[-1][0]:
                out.append("<%s>" % tag)
                stack.append((indent, tag))
            else:
                out.append("</li>")
            out.append("<li>%s" % self.inline(text))
            self.text.append(self.plain(text))
        while stack:
            out.append("</li></%s>" % stack.pop()[1])
        return "".join(out)


# ------------------------------------------------------------------ site

CSS = r"""
:root{--bg:#ffffff;--fg:#1d2330;--muted:#5b6577;--line:#dfe3ea;--side:#f5f7fa;--accent:#1f6fd1;--accent-bg:#e7f0fc;--code:#f0f2f5;--note:#fff8e0;--note-line:#e9c45a;--tip:#e8f6ec;--tip-line:#55a96b;color-scheme:light}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){--bg:#16191f;--fg:#e3e7ee;--muted:#9aa4b5;--line:#2c323d;--side:#1c2028;--accent:#6aa8ff;--accent-bg:#1f2b3d;--code:#232833;--note:#2f2a17;--note-line:#8f7425;--tip:#18291d;--tip-line:#3f8a52;color-scheme:dark}}
:root[data-theme="dark"]{--bg:#16191f;--fg:#e3e7ee;--muted:#9aa4b5;--line:#2c323d;--side:#1c2028;--accent:#6aa8ff;--accent-bg:#1f2b3d;--code:#232833;--note:#2f2a17;--note-line:#8f7425;--tip:#18291d;--tip-line:#3f8a52;color-scheme:dark}
*{box-sizing:border-box}
html,body{margin:0;padding:0}
body{background:var(--bg);color:var(--fg);font:16px/1.6 system-ui,-apple-system,"Segoe UI","Noto Sans",sans-serif}
a{color:var(--accent);text-decoration:none}a:hover{text-decoration:underline}
header.top{position:sticky;top:0;z-index:20;display:flex;align-items:center;gap:12px;padding:8px 16px;background:var(--bg);border-bottom:1px solid var(--line)}
header.top .brand{font-weight:650;color:var(--fg);white-space:nowrap;display:flex;align-items:center;gap:8px}
header.top .brand img{width:24px;height:24px}
header.top .spacer{flex:1}
header.top select,header.top button{font:inherit;font-size:14px;color:var(--fg);background:var(--side);border:1px solid var(--line);border-radius:6px;padding:4px 8px;cursor:pointer}
#menu-toggle{display:none}
.layout{display:grid;grid-template-columns:300px minmax(0,1fr);min-height:calc(100vh - 49px)}
aside.side{background:var(--side);border-inline-end:1px solid var(--line);position:sticky;top:49px;height:calc(100vh - 49px);overflow:auto;padding:12px 0 24px}
.tabs{display:flex;gap:4px;padding:0 12px 8px}
.tabs button{flex:1;font:inherit;font-size:14px;padding:6px;border:1px solid var(--line);background:var(--bg);color:var(--fg);border-radius:6px;cursor:pointer}
.tabs button[aria-selected="true"]{background:var(--accent-bg);border-color:var(--accent);color:var(--accent);font-weight:600}
nav.toc ul{list-style:none;margin:0;padding:0}
nav.toc li a{display:block;padding:3px 16px 3px 30px;color:var(--fg);font-size:14.5px;border-inline-start:3px solid transparent}
nav.toc li a:hover{background:var(--accent-bg);text-decoration:none}
nav.toc li a.current{border-inline-start-color:var(--accent);background:var(--accent-bg);color:var(--accent);font-weight:600}
nav.toc details>summary{list-style:none;cursor:pointer;padding:5px 12px;font-weight:600;font-size:14.5px;display:flex;align-items:center;gap:6px}
nav.toc details>summary::-webkit-details-marker{display:none}
nav.toc details>summary::before{content:"";width:0;height:0;border-style:solid;border-width:5px 0 5px 7px;border-color:transparent transparent transparent var(--muted);transition:transform .15s}
[dir="rtl"] nav.toc details>summary::before{border-width:5px 7px 5px 0;border-color:transparent var(--muted) transparent transparent}
nav.toc details[open]>summary::before{transform:rotate(90deg)}
[dir="rtl"] nav.toc details[open]>summary::before{transform:rotate(-90deg)}
nav.toc details>summary a{color:var(--fg);display:inline;padding:0;border:0;font-size:inherit}
nav.toc details>summary a:hover{background:none}
nav.toc details>summary a.current{color:var(--accent)}
#search-panel{padding:0 12px}
#search-panel input{width:100%;font:inherit;padding:7px 10px;border:1px solid var(--line);border-radius:6px;background:var(--bg);color:var(--fg)}
#results{list-style:none;padding:0;margin:10px 0}
#results li{padding:6px 4px;border-bottom:1px solid var(--line)}
#results li small{display:block;color:var(--muted);font-size:12.5px}
main{padding:20px 40px 60px;max-width:900px;width:100%;min-width:0}
.crumbs{font-size:13.5px;color:var(--muted);margin-bottom:6px}
.crumbs a{color:var(--muted)}
.crumbs .sep{padding:0 6px}
article h1{font-size:30px;line-height:1.25;margin:4px 0 18px}
article h2{font-size:22px;margin:34px 0 10px;padding-top:6px;border-top:1px solid var(--line)}
article h3{font-size:18px;margin:24px 0 8px}
article h4{font-size:16px;margin:18px 0 6px}
article h2,article h3,article h4{scroll-margin-top:64px}
article .anchor{visibility:hidden;margin-inline-start:8px;font-weight:400;color:var(--muted)}
article h2:hover .anchor,article h3:hover .anchor,article h4:hover .anchor{visibility:visible}
article code{background:var(--code);padding:1px 5px;border-radius:4px;font-size:.92em}
article pre{background:var(--code);padding:12px;border-radius:8px;overflow:auto}
article pre code{background:none;padding:0}
article kbd{font:600 .85em/1 ui-monospace,monospace;border:1px solid var(--line);border-bottom-width:2px;border-radius:4px;padding:2px 5px;background:var(--side)}
article .ui{font-weight:600}
article .path .sep{color:var(--muted);font-weight:400}
article .note,article .tip{border-inline-start:4px solid var(--note-line);background:var(--note);padding:10px 14px;border-radius:6px;margin:14px 0}
article .tip{border-color:var(--tip-line);background:var(--tip)}
article .table{overflow-x:auto}
article figure.shot{margin:16px 0}
article figure.shot img{max-width:100%;height:auto;display:block}
article table{border-collapse:collapse;margin:12px 0;font-size:15px}
article th,article td{border:1px solid var(--line);padding:6px 10px;text-align:start;vertical-align:top}
article th{background:var(--side)}
article ul.children{columns:2;column-gap:32px}
.untranslated{background:var(--accent-bg);border:1px solid var(--line);padding:8px 12px;border-radius:6px;font-size:14px;margin-bottom:14px}
.pager{display:flex;justify-content:space-between;gap:12px;margin-top:40px;padding-top:16px;border-top:1px solid var(--line)}
.pager a{display:block;max-width:48%;padding:8px 12px;border:1px solid var(--line);border-radius:8px}
.pager a small{display:block;color:var(--muted);font-size:12.5px}
.pager .next{margin-inline-start:auto;text-align:end}
footer{color:var(--muted);font-size:13px;margin-top:30px}
@media (max-width:820px){
 #menu-toggle{display:inline-block}
 .layout{grid-template-columns:minmax(0,1fr)}
 aside.side{position:fixed;inset-inline-start:0;top:49px;width:min(320px,88vw);z-index:15;transform:translateX(-105%);transition:transform .2s;box-shadow:0 6px 24px rgba(0,0,0,.25)}
 [dir="rtl"] aside.side{transform:translateX(105%)}
 body.menu-open aside.side{transform:none}
 main{padding:16px 16px 48px}
 article ul.children{columns:1}
 header.top .brand span{display:none}
}
@media print{header.top,aside.side,.pager{display:none}.layout{display:block}}
"""

JS = r"""
(function(){
var root=document.documentElement;
try{var t=localStorage.getItem("td-help-theme");if(t)root.setAttribute("data-theme",t);}catch(e){}
document.getElementById("theme").addEventListener("click",function(){
 var dark=root.getAttribute("data-theme")==="dark"||(!root.getAttribute("data-theme")&&matchMedia("(prefers-color-scheme: dark)").matches);
 var n=dark?"light":"dark";root.setAttribute("data-theme",n);try{localStorage.setItem("td-help-theme",n);}catch(e){}
});
document.getElementById("menu-toggle").addEventListener("click",function(){document.body.classList.toggle("menu-open");});
var lang=document.getElementById("lang");
lang.addEventListener("change",function(){
 location.href=HELP.up+"../"+lang.value+"/"+HELP.page+".html"+location.hash;
});
var tabs=document.querySelectorAll(".tabs button"),toc=document.getElementById("toc"),panel=document.getElementById("search-panel"),q=document.getElementById("q"),res=document.getElementById("results");
function show(which){
 tabs.forEach(function(b){b.setAttribute("aria-selected",String(b.dataset.tab===which));});
 toc.hidden=which!=="contents";panel.hidden=which!=="search";
 if(which==="search"){q.focus();load();}
 try{sessionStorage.setItem("td-help-tab",which);}catch(e){}
}
tabs.forEach(function(b){b.addEventListener("click",function(){show(b.dataset.tab);});});
var index=null,loading=false;
function load(){
 if(index||loading)return;loading=true;
 fetch(HELP.up+"search.json").then(function(r){return r.json();}).then(function(j){index=j;run();}).catch(function(){loading=false;});
}
function norm(s){return s.toLowerCase().normalize("NFKD").replace(/[̀-ͯ]/g,"");}
function run(){
 if(!index)return;
 var terms=norm(q.value).split(/\s+/).filter(Boolean);
 res.innerHTML="";
 if(!terms.length)return;
 var hits=[];
 index.forEach(function(p){
  var t=norm(p.t),h=norm(p.h),x=norm(p.x),score=0;
  for(var i=0;i<terms.length;i++){
   var w=terms[i],s=0;
   if(t.indexOf(w)>=0)s+=20;if(h.indexOf(w)>=0)s+=6;
   var k=0,pos=x.indexOf(w);while(pos>=0&&k<20){k++;pos=x.indexOf(w,pos+w.length);}s+=k;
   if(!s)return;score+=s;
  }
  hits.push([score,p]);
 });
 hits.sort(function(a,b){return b[0]-a[0];});
 if(!hits.length){var li=document.createElement("li");li.textContent=HELP.none;res.appendChild(li);return;}
 hits.slice(0,40).forEach(function(e){
  var p=e[1],li=document.createElement("li"),a=document.createElement("a");
  a.href=HELP.up+p.p+".html";a.textContent=p.t;li.appendChild(a);
  var s=document.createElement("small");s.textContent=p.s;li.appendChild(s);res.appendChild(li);
 });
}
q.addEventListener("input",run);
try{if(sessionStorage.getItem("td-help-tab")==="search"){show("search");}}catch(e){}
var cur=document.querySelector("nav.toc a.current");if(cur&&cur.scrollIntoView)cur.scrollIntoView({block:"center"});
})();
"""


def esc(s):
    return html.escape(s, quote=True)


def build(help_dir, lang_dir, out_dir):
    langs = languages()
    if ONLY:
        langs = [l for l in langs if l[0] in ONLY]
    tree = read_contents(os.path.join(help_dir, "contents.txt"))
    for sec, kids in tree:
        ALL_LISTED.add(sec)
        ALL_LISTED.update(kids)
    order = []
    parent = {}
    for sec, kids in tree:
        order.append(sec)
        for k in kids:
            order.append(k)
            parent[k] = sec
    en_table = read_lang(os.path.join(lang_dir, "en.toml"))
    missing = [p for p in order if not os.path.exists(os.path.join(help_dir, "en", p + ".md"))]
    if PARTIAL:
        # While pages are being written: leave out the missing ones.
        order = [p for p in order if p not in missing]
        tree = [(s, [k for k in kids if k not in missing]) for s, kids in tree if s not in missing]
        if missing:
            print("partial: %d page(s) not written yet" % len(missing))
    else:
        for p in missing:
            err("en/%s.md is listed in contents.txt but missing" % p)
    if ERRORS:
        return
    if os.path.isdir(out_dir):
        shutil.rmtree(out_dir)
    os.makedirs(out_dir)
    with open(os.path.join(out_dir, "help.css"), "w", encoding="utf-8") as f:
        f.write(CSS.strip() + "\n")
    with open(os.path.join(out_dir, "help.js"), "w", encoding="utf-8") as f:
        f.write(JS.strip() + "\n")
    icon = os.path.join(ROOT, "assets/icon/tracedraw-256.png")
    if os.path.exists(icon):
        shutil.copy(icon, os.path.join(out_dir, "icon.png"))
    img_src = os.path.join(help_dir, "img")
    if os.path.isdir(img_src):
        os.makedirs(os.path.join(out_dir, "img"))
        for f in sorted(os.listdir(img_src)):
            if f.endswith(".png"):
                shutil.copy(os.path.join(img_src, f), os.path.join(out_dir, "img", f))
    stats = {}
    for code, native in langs:
        table = read_lang(os.path.join(lang_dir, code + ".toml"))
        ui = lambda k: table.get(k) or en_table.get(k) or k
        pages = {}
        translated = 0
        for p in order:
            src = os.path.join(help_dir, code, p + ".md")
            fallback = not os.path.exists(src)
            if fallback:
                src = os.path.join(help_dir, "en", p + ".md")
            else:
                translated += 1
            md = open(src, encoding="utf-8").read()
            pages[p] = Page(p, code, md, en_table if fallback else table, en_table, fallback)
        stats[code] = translated
        # Links, anchors and images.
        for p, pg in pages.items():
            for im in pg.images:
                if not os.path.exists(os.path.join(help_dir, im)):
                    err("%s/%s: missing image %s" % (code, p, im))
            for target, anchor in pg.links:
                if target not in pages:
                    if PARTIAL and target in ALL_LISTED:
                        continue
                    err("%s/%s: link to a missing page %s" % (code, p, target))
                elif anchor and anchor not in {h[1] for h in pages[target].headings}:
                    err("%s/%s: link to a missing anchor %s#%s" % (code, p, target, anchor))
        index = []
        for n, p in enumerate(order):
            pg = pages[p]
            up = "../" * p.count("/")
            index.append(
                {
                    "p": p,
                    "t": pg.title,
                    "s": pages[parent[p]].title if p in parent else ui("helpsite.title"),
                    "h": " ".join(h[2] for h in pg.headings),
                    "x": " ".join(pg.text)[:6000],
                }
            )
            html_out = page_html(code, langs, ui, tree, pages, order, n, up)
            dest = os.path.join(out_dir, code, p + ".html")
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "w", encoding="utf-8") as f:
                f.write(html_out)
        os.makedirs(os.path.join(out_dir, code), exist_ok=True)
        with open(os.path.join(out_dir, code, "search.json"), "w", encoding="utf-8") as f:
            json.dump(index, f, ensure_ascii=False, separators=(",", ":"))
    codes = [c for c, _ in langs]
    with open(os.path.join(out_dir, "index.html"), "w", encoding="utf-8") as f:
        f.write(redirect_html(codes))
    with open(os.path.join(out_dir, "languages.json"), "w", encoding="utf-8") as f:
        json.dump({"pages": len(order), "translated": stats}, f, indent=1)
    return stats, len(order)


def toc_html(tree, pages, current):
    out = ['<nav class="toc" id="toc"><ul>']
    for sec, kids in tree:
        cur = ' class="current"' if sec == current else ""
        is_open = sec == current or current in kids
        out.append(
            '<li><details%s><summary><a href="{up}%s.html"%s>%s</a></summary><ul>'
            % (" open" if is_open else "", sec, cur, esc(pages[sec].title))
        )
        for k in kids:
            c = ' class="current" aria-current="page"' if k == current else ""
            out.append('<li><a href="{up}%s.html"%s>%s</a></li>' % (k, c, esc(pages[k].title)))
        out.append("</ul></details></li>")
    out.append("</ul></nav>")
    return "".join(out)


def page_html(code, langs, ui, tree, pages, order, n, up):
    p = order[n]
    pg = pages[p]
    sec_of = {}
    for sec, kids in tree:
        for k in kids:
            sec_of[k] = sec
    crumbs = ['<a href="%sindex.html">%s</a>' % (up, esc(ui("helpsite.title")))]
    if p in sec_of:
        crumbs.append('<a href="%s%s.html">%s</a>' % (up, sec_of[p], esc(pages[sec_of[p]].title)))
    if p != "index":
        crumbs.append("<span>%s</span>" % esc(pg.title))
    body = pg.body
    kids = dict(tree).get(p)
    if kids:
        items = "".join('<li><a href="%s%s.html">%s</a></li>' % (up, k, esc(pages[k].title)) for k in kids)
        body += '\n<ul class="children">%s</ul>' % items
    notice = '<p class="untranslated">%s</p>' % esc(ui("helpsite.untranslated")) if pg.fallback and code != "en" else ""
    prev_link = next_link = ""
    if n > 0:
        q = order[n - 1]
        prev_link = '<a class="prev" href="%s%s.html" rel="prev"><small>%s</small>%s</a>' % (
            up, q, esc(ui("helpsite.previous")), esc(pages[q].title))
    if n + 1 < len(order):
        q = order[n + 1]
        next_link = '<a class="next" href="%s%s.html" rel="next"><small>%s</small>%s</a>' % (
            up, q, esc(ui("helpsite.next")), esc(pages[q].title))
    options = "".join(
        '<option value="%s"%s>%s</option>' % (c, " selected" if c == code else "", esc(name)) for c, name in langs
    )
    toc = toc_html(tree, pages, p).replace("{up}", up)
    rtl = ' dir="rtl"' if code in RTL else ' dir="ltr"'
    hreflang = "".join(
        '<link rel="alternate" hreflang="%s" href="%s../%s/%s.html">' % (c, up, c, p) for c, _ in langs
    )
    page_lang = "en" if pg.fallback else code
    cfg = json.dumps({"page": p, "up": up, "none": ui("helpsite.no_results")}, ensure_ascii=False)
    title = esc(pg.title) if p == "index" else "%s - %s" % (esc(pg.title), esc(ui("helpsite.title")))
    return f"""<!doctype html>
<html lang="{code}"{rtl}>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="{up}../help.css">
<link rel="icon" href="{up}../icon.png">
{hreflang}
<script>try{{var t=localStorage.getItem("td-help-theme");if(t)document.documentElement.setAttribute("data-theme",t);}}catch(e){{}}</script>
</head>
<body>
<header class="top">
<button id="menu-toggle" type="button" aria-label="{esc(ui("helpsite.show_contents"))}">&#9776;</button>
<a class="brand" href="{up}index.html"><img src="{up}../icon.png" alt=""><span>{esc(ui("helpsite.title"))}</span></a>
<span class="spacer"></span>
<label><span hidden>{esc(ui("helpsite.language"))}</span><select id="lang" aria-label="{esc(ui("helpsite.language"))}">{options}</select></label>
<button id="theme" type="button" title="{esc(ui("helpsite.theme"))}" aria-label="{esc(ui("helpsite.theme"))}">&#9680;</button>
</header>
<div class="layout">
<aside class="side">
<div class="tabs" role="tablist"><button type="button" role="tab" data-tab="contents" aria-selected="true">{esc(ui("helpsite.contents"))}</button><button type="button" role="tab" data-tab="search" aria-selected="false">{esc(ui("helpsite.search"))}</button></div>
{toc}
<div id="search-panel" hidden><input id="q" type="search" placeholder="{esc(ui("helpsite.search_placeholder"))}" aria-label="{esc(ui("helpsite.search"))}"><ul id="results"></ul></div>
</aside>
<main>
<div class="crumbs">{'<span class="sep">/</span>'.join(crumbs)}</div>
<article lang="{page_lang}"{' dir="ltr"' if pg.fallback and code in RTL else ''}>
{notice}{body}
</article>
<nav class="pager">{prev_link}{next_link}</nav>
<footer>TraceDraw</footer>
</main>
</div>
<script>var HELP={cfg};</script>
<script src="{up}../help.js"></script>
</body>
</html>
"""


def redirect_html(codes):
    return """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>TraceDraw Help</title>
<script>
(function(){var codes=%s,pick="en";
var want=(navigator.languages||[navigator.language||"en"]);
for(var i=0;i<want.length&&pick==="en";i++){var w=want[i];
 for(var j=0;j<codes.length;j++){if(codes[j].toLowerCase()===w.toLowerCase()){pick=codes[j];break;}}
 if(pick!=="en")break;
 var base=w.split("-")[0].toLowerCase();
 if(base==="zh"){pick=/tw|hk|mo|hant/i.test(w)?"zh-TW":"zh-CN";break;}
 if(base==="pt"){pick="pt-BR";break;}
 if(base!=="en"&&codes.indexOf(base)>=0){pick=base;break;}
 if(base==="en")break;}
location.replace(pick+"/index.html"+location.hash);})();
</script>
<noscript><meta http-equiv="refresh" content="0; url=en/index.html"></noscript>
</head><body><a href="en/index.html">TraceDraw Help</a></body></html>
""" % json.dumps(codes)


def main():
    global PARTIAL
    args = []
    for a in sys.argv[1:]:
        if a == "--partial":
            PARTIAL = True
        elif a.startswith("--lang="):
            ONLY.update(a[7:].split(","))
        else:
            args.append(a)
    help_dir = args[0] if len(args) > 0 else os.path.join(ROOT, "docs/help")
    lang_dir = args[1] if len(args) > 1 else os.path.join(ROOT, "apps/tracedraw/lang")
    out_dir = args[2] if len(args) > 2 else os.path.join(ROOT, "site/help")
    result = build(help_dir, lang_dir, out_dir)
    if ERRORS:
        for e in ERRORS[:200]:
            print("error:", e, file=sys.stderr)
        print("%d error(s)" % len(ERRORS), file=sys.stderr)
        sys.exit(1)
    stats, total = result
    print("Help: %d pages in %d languages -> %s" % (total, len(stats), out_dir))
    for code, n in stats.items():
        print("  %-6s %3d/%d translated" % (code, n, total))


if __name__ == "__main__":
    main()

# Automated fix for issue #761: Edit fill Color change problem
