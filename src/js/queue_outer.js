(function() {
    var BROWSER_NAMES = ['Google Chrome', 'Chromium', 'Brave Browser', 'Arc', 'Microsoft Edge', 'Opera'];
    var deezerTab = null;
    for (var b = 0; b < BROWSER_NAMES.length; b++) {
        var browser;
        try { browser = Application(BROWSER_NAMES[b]); } catch(e) { continue; }
        try {
            var wins = browser.windows();
            for (var i = 0; i < wins.length; i++) {
                try {
                    var tabs = wins[i].tabs();
                    for (var j = 0; j < tabs.length; j++) {
                        if (tabs[j].url().indexOf('deezer.com') !== -1) {
                            deezerTab = tabs[j];
                            break;
                        }
                    }
                } catch(e) {}
                if (deezerTab) break;
            }
        } catch(e) {}
        if (deezerTab) break;
    }
    if (!deezerTab) return "ERROR:NO_DEEZER_TAB";

    // Clear any previous state
    deezerTab.execute({javascript: "['__dz_running','__deezranbum','__deezranbum_logs'].forEach(function(k){localStorage.removeItem(k);});"});

    // Run queue logic directly — execute() bypasses CSP, no <script> injection needed.
    try {
        deezerTab.execute({javascript: __MAIN_WORLD_JS_JSON__});
    } catch(e) {
        return "ERROR:EXECUTE_FAILED:" + String(e);
    }

    // Poll for result (up to ~18s)
    delay(0.5);
    var initCheck = deezerTab.execute({javascript: "localStorage.getItem('__deezranbum')"});

    for (var poll = 0; poll < 60; poll++) {
        delay(0.3);
        var r = deezerTab.execute({javascript: "localStorage.getItem('__deezranbum')"});
        if (r && r !== "null") {
            deezerTab.execute({javascript: "['__dz_running','__deezranbum','__deezranbum_logs'].forEach(function(k){localStorage.removeItem(k);});"});
            return "OK\n" + r;
        }
    }

    var logs = deezerTab.execute({javascript: "localStorage.getItem('__deezranbum_logs')"});
    return "ERROR:TIMEOUT:initLS=" + String(initCheck) + ",logs=" + String(logs);
})();
