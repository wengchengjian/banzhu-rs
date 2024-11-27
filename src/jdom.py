import functools
import subprocess

subprocess.Popen = functools.partial(subprocess.Popen, encoding="utf-8")
# 不加上两行会导致execjs返回值出现编码错误
import execjs

# 读取js文件
with open(r'asset/js/a.js') as f:
    a_js = f.read()

def get_section_data_by_js(*args, **kwargs):

    section_html = args[0]
    ns = args[1]

    full_a_js = '''
        const jsdom = require("jsdom");
        const { JSDOM } = jsdom;
        const dom = new JSDOM(`%s`);
        window = dom.window;
        document = window.document;
        XMLHttpRequest = window.XMLHttpRequest;
        %s
        function get_data(ns) {
            _ii_rr(ns);
            let ad = dom.window.document.querySelector("#ad");
            if(ad != null && ad.innerHTML != null) {
                return ad.innerHTML;
            }
            return ''
        }
        ''' % (section_html.replace('\n', '').replace('`', '\\`'), a_js)
    ctx = execjs.compile(full_a_js)
    content = ctx.call("get_data", ns)
    return content
