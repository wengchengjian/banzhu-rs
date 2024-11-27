from DrissionPage import ChromiumPage, ChromiumOptions
from fake_useragent import UserAgent

ua = UserAgent()

options = ChromiumOptions()
args = [
    "-no-first-run",
    "-force-color-profile=srgb",
    "-metrics-recording-only",
    "-password-store=basic",
    "-use-mock-keychain",
    "-export-tagged-pdf",
    "-no-default-browser-check",
    "-disable-background-mode",
    "-enable-features=NetworkService,NetworkServiceInProcess,LoadCryptoTokenExtension,PermuteTLSExtensions",
    "-disable-features=FlashDeprecationWarning,EnablePasswordsAccountStorage",
    "-deny-permission-prompts",
    #"--start-fullscreen",
]
for arg in args:
    options.set_argument(arg)

# Mozilla/5.0 (iPhone; CPU iPhone OS 17_0_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1
options.set_user_agent(ua.random)
driver = ChromiumPage(addr_or_opts=options)

def open_url(url):
    driver.get(url)

def screenshot():
    return driver.get_screenshot(as_bytes='jpg')

def _cookie_format_convert(driver_cookie):
    requests_cookie = ''
    for dict in driver_cookie:
        requests_cookie += f'{dict["name"]}={dict["value"]}; '
    return requests_cookie
def get_title():
    return driver.title

def get_page_location():
    return driver.rect.page_location

def get_ua():
    return driver.user_agent

def get_cookie():
    return _cookie_format_convert(driver.cookies())

def quit():
    driver.close()