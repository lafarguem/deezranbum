(function(){
if (localStorage.getItem('__dz_running')) return;
localStorage.setItem('__dz_running', 'yes');
try {
    var log = function(m) {
        var prev = localStorage.getItem('__deezranbum_logs') || '';
        localStorage.setItem('__deezranbum_logs', prev + m + '\n');
    };

    function findPlayer() {
        log('Searching for player');
        try {
            var dzGlobals = Object.keys(window).filter(function(k) {
                return k.startsWith('dz') || k.toLowerCase().includes('deezer') || k === 'pipe' || k === 'Events';
            });
            log('Deezer-ish globals: ' + (dzGlobals.join(', ') || 'none'));

            var dzp = window.dzPlayer;
            if (dzp) {
                log('Found window.dzPlayer');
                log('type: ' + typeof dzp);
                try { log('keys: ' + Object.keys(dzp).slice(0,30).join(', ')); } catch(e) {}
                return dzp;
            }

            var frames = document.querySelectorAll('iframe');
            log('Checking ' + frames.length + ' iframes');
            for (var i = 0; i < frames.length; i++) {
                try {
                    if (frames[i].contentWindow && frames[i].contentWindow.dzPlayer) {
                        log('Found dzPlayer in iframe ' + i);
                        return frames[i].contentWindow.dzPlayer;
                    }
                } catch(e) {}
            }
        } catch(e) { log('findPlayer error: ' + e); }
        return null;
    }

    async function main() {
        log('main() start, album_id=__ALBUM_ID__');
        for (var attempt = 0; attempt < 40; attempt++) {
            log('attempt ' + attempt);
            var dzp = findPlayer();
            if (dzp) {
                var hasEnqueue = typeof dzp.enqueueTracks === 'function';
                log('player found, enqueueTracks=' + hasEnqueue);
                if (!hasEnqueue) {
                    var queueMethods = Object.keys(dzp).filter(function(k) {
                        return k.toLowerCase().includes('queue') || k.toLowerCase().includes('enqueue') || k.toLowerCase().includes('track') || k.toLowerCase().includes('play');
                    });
                    log('queue-ish methods: ' + (queueMethods.join(', ') || 'none'));
                }
            }
            if (dzp && typeof dzp.enqueueTracks === 'function') {
                log('Player ready');
                try {
                    log('fetching user data...');
                    var udResp = await fetch(
                        '/ajax/gw-light.php?method=deezer.getUserData&input=3&api_version=1.0&api_token=null',
                        {credentials: 'include'}
                    );
                    log('getUserData status: ' + udResp.status);
                    var ud = await udResp.json();
                    var token = ud.results && ud.results.checkForm;
                    var userId = ud.results && ud.results.USER && ud.results.USER.USER_ID;
                    log('token present: ' + !!token + ', userId: ' + userId);
                    log('fetching album tracks...');
                    var ar = await fetch(
                        '/ajax/gw-light.php?method=deezer.pageAlbum&input=3&api_version=1.0&api_token='
                        + token + '&cid=' + Math.floor(Math.random() * 1e9),
                        {
                            method: 'POST', credentials: 'include',
                            headers: {'Content-Type': 'text/plain;charset=UTF-8', 'x-deezer-user': userId || ''},
                            body: JSON.stringify({alb_id: __ALBUM_ID__, lang: 'us', tab: 0, header: true})
                        }
                    );
                    log('pageAlbum status: ' + ar.status);
                    var ad = await ar.json();
                    var songs = ad && ad.results && ad.results.SONGS;
                    log('SONGS present: ' + !!songs + ', count: ' + (songs && songs.data ? songs.data.length : 'n/a'));
                    var tracks = songs && songs.data;
                    if (tracks && tracks.length) {
                        log('calling enqueueTracks with ' + tracks.length + ' tracks');
                        dzp.enqueueTracks(tracks, {object_type: 'album', object_id: '__ALBUM_ID__', radio: false});
                        log('enqueueTracks returned');
                        localStorage.setItem('__deezranbum', JSON.stringify({status: 'ok', logs: localStorage.getItem('__deezranbum_logs')}));
                        return;
                    }
                    log('no tracks found in response');
                } catch(e) { log('queue error: ' + e + ' stack: ' + (e && e.stack)); }
            }
            await new Promise(function(r) { setTimeout(r, 250); });
        }
        localStorage.setItem('__deezranbum', JSON.stringify({status: 'timeout', logs: localStorage.getItem('__deezranbum_logs')}));
    }

    main().catch(function(e) {
        log('main() unhandled rejection: ' + e);
        localStorage.setItem('__deezranbum', JSON.stringify({status: 'main-rejection', error: String(e), logs: localStorage.getItem('__deezranbum_logs')}));
    });
} catch(e) {
    localStorage.setItem('__deezranbum', JSON.stringify({status: 'main-world-error', error: String(e)}));
}
})();
