// Caterpillar cursor - follows mouse with pixel art animation
(function caterpillar(){
    const el = document.getElementById('caterpillar');
    if (!el) return;

    const ctx = el.getContext('2d');
    const bodyColor = '#97BAD9';
    const shadowColor = '#7BA3C9';
    const legColor = '#5a6a7a';

    // Detect touch device
    const isTouchDevice = 'ontouchstart' in window || navigator.maxTouchPoints > 0;

    // Restore state from sessionStorage
    const saved = JSON.parse(sessionStorage.getItem('caterpillar') || 'null');
    let posX = saved?.posX || 100, posY = saved?.posY || 100;
    let mouseX = posX, mouseY = posY;
    let velX = saved?.velX || 0, velY = saved?.velY || 0;
    let frame = saved?.frame || 0;
    let idleTime = saved?.idleTime || 0;
    let idleAnim = saved?.idleAnim || null;
    let idleFrame = saved?.idleFrame || 0;
    let lastState = '', lastFK = -1;
    let facingRight = saved?.facingRight !== false;
    let lastTS = 0;

    // Save state before leaving page
    window.addEventListener('beforeunload', () => {
        sessionStorage.setItem('caterpillar', JSON.stringify({
            posX, posY, velX, velY, frame, idleTime, idleAnim, idleFrame, facingRight
        }));
    });

    function draw(state, af) {
        const fk = ~~af % 4;
        if (state === lastState && fk === lastFK) return;
        lastState = state;
        lastFK = fk;
        ctx.clearRect(0, 0, 72, 40);

        const w = state === 'walk', sl = state === 'sleep', al = state === 'alert';
        const leg = fk % 2, wv = w ? Math.sin(fk * 1.5) : 0, bob = w ? Math.abs(wv) * 2 : sl ? 2 : 0;

        // Body segments
        for (let i = 0; i < 4; i++) {
            const sx = 4 + i * 12 + (w ? wv * i * .3 : 0);
            const sy = 18 + (w ? Math.sin(fk * 1.5 + (3 - i)) * 1.5 : 0);
            if (!sl) {
                ctx.fillStyle = legColor;
                ctx.fillRect(sx + 2, sy + 6 + (leg && i % 2 ? -1 : 0), 3, 4);
                ctx.fillRect(sx + 9, sy + 6 + (leg && i % 2 ? 0 : -1), 3, 4);
            }
            ctx.fillStyle = bodyColor;
            ctx.fillRect(sx, sy, 14, 10);
            ctx.fillStyle = shadowColor;
            ctx.fillRect(sx, sy, 14, 2);
            ctx.fillRect(sx, sy + 8, 14, 2);
        }

        // Head
        const hx = 52, hy = 16 - bob;
        ctx.fillStyle = legColor;
        ctx.fillRect(hx + 4, hy - 6, 2, 6);
        ctx.fillRect(hx + 10, hy - 6, 2, 6);
        ctx.fillStyle = bodyColor;
        ctx.beginPath();
        ctx.arc(hx + 5, hy - 8, 3, 0, 6.28);
        ctx.arc(hx + 11, hy - 8, 3, 0, 6.28);
        ctx.fill();
        ctx.fillRect(hx, hy, 16, 12);
        ctx.fillStyle = shadowColor;
        ctx.fillRect(hx, hy, 16, 2);
        ctx.fillRect(hx, hy + 10, 16, 2);
        if (!sl) {
            ctx.fillStyle = legColor;
            ctx.fillRect(hx + 3, hy + 10, 3, 4);
            ctx.fillRect(hx + 10, hy + 10, 3, 4);
        }

        // Eyes
        ctx.fillStyle = '#1a1a1a';
        if (sl) {
            ctx.fillRect(hx + 3, hy + 4, 4, 2);
            ctx.fillRect(hx + 9, hy + 4, 4, 2);
            ctx.font = '8px sans-serif';
            ctx.fillText('z', hx + 18, hy - 2 + (fk % 2 ? 0 : -2));
        } else if (al) {
            ctx.fillRect(hx + 3, hy + 3, 5, 5);
            ctx.fillRect(hx + 9, hy + 3, 5, 5);
        } else {
            ctx.fillRect(hx + 4, hy + 3, 3, 4);
            ctx.fillRect(hx + 10, hy + 3, 3, 4);
        }
    }

    // Desktop: follow mouse
    if (!isTouchDevice) {
        document.addEventListener('mousemove', e => {
            mouseX = e.clientX;
            mouseY = e.clientY;
        });
    }

    // Mobile: only move when tapping interactive elements
    if (isTouchDevice) {
        document.addEventListener('click', e => {
            const target = e.target.closest('a, button, [role="button"], .btn, .product-card');
            if (target) {
                const rect = target.getBoundingClientRect();
                mouseX = rect.left + rect.width / 2;
                mouseY = rect.top + rect.height / 2;
                idleTime = 0;
                idleAnim = null;
            }
        });
    }

    function update(ts) {
        const dt = lastTS ? (ts - lastTS) / 1000 : .016;
        lastTS = ts;
        frame += dt * 8;
        const dx = mouseX - posX, dy = mouseY - posY;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist < 48) {
            velX *= .9;
            velY *= .9;
            idleTime += dt;
            if (idleTime > 8 && !idleAnim && Math.random() < .005) {
                idleAnim = 'sleep';
                idleFrame = 0;
            }
            if (idleAnim === 'sleep') {
                idleFrame += dt * 4;
                draw('sleep', idleFrame);
                if (idleFrame > 80) idleAnim = null;
            } else {
                draw('idle', frame);
            }
        } else if (idleTime > .05) {
            draw('alert', 0);
            idleTime = Math.max(0, idleTime - dt * 8);
        } else {
            idleAnim = null;
            velX += (dx / dist * 150 - velX) * .12;
            velY += (dy / dist * 150 - velY) * .12;
            posX += velX * dt;
            posY += velY * dt;
            posX = Math.max(36, Math.min(posX, innerWidth - 36));
            posY = Math.max(20, Math.min(posY, innerHeight - 20));
            if (Math.abs(velX) > 5) facingRight = velX > 0;
            draw('walk', frame);
        }

        el.style.transform = `translate3d(${posX - 36}px,${posY - 20}px,0)scaleX(${facingRight ? 1 : -1})`;
        requestAnimationFrame(update);
    }

    draw('idle', 0);
    requestAnimationFrame(update);
})();
