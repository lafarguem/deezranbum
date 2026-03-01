(function() {
    var code = __MAIN_WORLD_JS_JSON__;
    var blob = new Blob([code], {type: 'application/javascript'});
    var url = URL.createObjectURL(blob);
    var s = document.createElement('script');
    s.src = url;
    document.documentElement.appendChild(s);
})();
