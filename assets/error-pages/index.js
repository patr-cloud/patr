// Procedural twinkling stars
(function () {
  var c = document.getElementById("stars");
  for (var i = 0; i < 25; i++) {
    var s = document.createElement("div");
    s.className = "star";
    s.style.left = Math.random() * 100 + "%";
    s.style.top = Math.random() * 100 + "%";
    var sz = Math.random() * 5;
    s.style.width = sz + "px";
    s.style.height = sz + "px";
    s.style.setProperty("--dur", (Math.random() * 2 + 1.5) + "s");
    s.style.setProperty("--peak", (0.4 + Math.random() * 0.5).toFixed(2));
    s.style.animationDelay = Math.random() * 3 + "s";
    c.appendChild(s);
  }
})();
