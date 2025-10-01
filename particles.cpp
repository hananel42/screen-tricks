#include <windows.h>
#include <vector>
#include <cstdlib>
#include <ctime>

// Particle cached small bitmap + physics
struct Particle {
    HBITMAP bmp;
    int w, h;
    double x, y;
    double vx, vy;
    double delay;
    bool alive;
};

int screenW = 0, screenH = 0;
HBITMAP hSrcBmp = nullptr;
std::vector<Particle> particles;
double gravity = 800.0; // increase for snappier fall

void SetDPIAwareness() {
    HMODULE hUser32 = LoadLibraryA("user32.dll");
    if (hUser32) {
        typedef BOOL(WINAPI* SetProcessDPIAwareFunc)();
        SetProcessDPIAwareFunc pFunc = (SetProcessDPIAwareFunc)GetProcAddress(hUser32, "SetProcessDPIAware");
        if (pFunc) pFunc();
        FreeLibrary(hUser32);
    }
}

HBITMAP CaptureScreen(int &outW, int &outH) {
    HDC hScreen = GetDC(NULL);
    if (!hScreen) return nullptr;
    outW = GetDeviceCaps(hScreen, HORZRES);
    outH = GetDeviceCaps(hScreen, VERTRES);

    HDC memDC = CreateCompatibleDC(hScreen);
    HBITMAP bmp = CreateCompatibleBitmap(hScreen, outW, outH);
    HBITMAP oldBmp = (HBITMAP)SelectObject(memDC, bmp);
    BitBlt(memDC, 0, 0, outW, outH, hScreen, 0, 0, SRCCOPY | CAPTUREBLT);
    SelectObject(memDC, oldBmp);
    DeleteDC(memDC);
    ReleaseDC(NULL, hScreen);
    return bmp;
}
void InitParticles(HDC hdcSrc, int pw = 16, int ph = 16) {
    particles.clear();
    srand((unsigned)time(NULL));

    // התחלה מלמטה למעלה
    for (int y = screenH - ph; y >= 0; y -= ph) {
        for (int x = 0; x < screenW; x += pw) {
            Particle p;
            p.w = pw; p.h = ph;
            p.x = (double)x; 
            p.y = (double)y;
            p.vx = (rand() % 100 - 50) / 40.0;
            p.vy = (rand() % 80) / 80.0;
            // דיליי - עכשיו מלמטה למעלה
            p.delay = (screenH - y) * 0.006 + (rand() % 100) * 0.005;
            p.alive = true;

            p.bmp = CreateCompatibleBitmap(hdcSrc, pw, ph);
            HDC partDC = CreateCompatibleDC(hdcSrc);
            HBITMAP old = (HBITMAP)SelectObject(partDC, p.bmp);
            BitBlt(partDC, 0, 0, pw, ph, hdcSrc, x, y, SRCCOPY);
            SelectObject(partDC, old);
            DeleteDC(partDC);

            particles.push_back(p);
        }
    }
}

LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    if (msg == WM_DESTROY) {
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProc(hwnd, msg, wParam, lParam);
}

int WINAPI WinMain(HINSTANCE hInst, HINSTANCE, LPSTR, int) {
    // DPI first
    SetDPIAwareness();

    // capture screen BEFORE creating window (we want the desktop image)
    hSrcBmp = CaptureScreen(screenW, screenH);
    if (!hSrcBmp) return 1;

    const char CLASS_NAME[] = "ShatterScreenFast";
    WNDCLASSA wc = {};
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInst;
    wc.lpszClassName = CLASS_NAME;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(
        WS_EX_TOPMOST,
        CLASS_NAME, "Shatter",
        WS_POPUP | WS_VISIBLE,
        0, 0, screenW, screenH,
        NULL, NULL, hInst, NULL
    );
    if (!hwnd) {
        DeleteObject(hSrcBmp);
        return 1;
    }

    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);

    // DCs
    HDC hdcWindow = GetDC(hwnd);
    HDC hdcSrc = CreateCompatibleDC(hdcWindow);
    HBITMAP oldSrc = (HBITMAP)SelectObject(hdcSrc, hSrcBmp);

    // prepare cached particle bitmaps
    InitParticles(hdcSrc, 16, 16); // increase fragment size for perf; lower -> finer

    // create back buffer once
    HDC hdcBack = CreateCompatibleDC(hdcWindow);
    HBITMAP hbmBack = CreateCompatibleBitmap(hdcWindow, screenW, screenH);
    HBITMAP oldBack = (HBITMAP)SelectObject(hdcBack, hbmBack);

    // create a DC for drawing particle bitmaps (reused each frame)
    HDC memDC = CreateCompatibleDC(hdcBack);

    // timing
    ULONGLONG prev = GetTickCount64();
    double timeElapsed = 0.0;

    MSG msg;
    bool running = true;
    while (running) {
        while (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_QUIT) { running = false; break; }
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
        if (!running) break;

        ULONGLONG now = GetTickCount64();
        double dt = (now - prev) / 1000.0;
        if (dt > 0.05) dt = 0.05; // clamp big dt
        prev = now;
        timeElapsed += dt;

        // draw to back buffer
        RECT rc = { 0,0, screenW, screenH };
        HBRUSH black = (HBRUSH)GetStockObject(BLACK_BRUSH);
        FillRect(hdcBack, &rc, black);

        // draw particles into back buffer using cached bitmaps
        for (auto &p : particles) {
            if (!p.alive) continue;

            if (timeElapsed < p.delay) {
                // still at original place
                HBITMAP old = (HBITMAP)SelectObject(memDC, p.bmp);
                BitBlt(hdcBack, (int)p.x, (int)p.y, p.w, p.h, memDC, 0, 0, SRCCOPY);
                SelectObject(memDC, old);
                continue;
            }

            // update physics (use dt for both axes)
            p.vy += gravity * dt;
            p.x += p.vx * dt * 60.0; // scale vx to be visible (matching previous behavior)
            p.y += p.vy * dt;

            if (p.y >= screenH) { p.alive = false; continue; }

            HBITMAP old = (HBITMAP)SelectObject(memDC, p.bmp);
            BitBlt(hdcBack, (int)p.x, (int)p.y, p.w, p.h, memDC, 0, 0, SRCCOPY);
            SelectObject(memDC, old);
        }

        // flip back buffer to window (single fast BitBlt)
        BitBlt(hdcWindow, 0, 0, screenW, screenH, hdcBack, 0, 0, SRCCOPY);

        Sleep(16); // ~60 FPS
    }

    // cleanup
    SelectObject(hdcSrc, oldSrc);
    DeleteDC(hdcSrc);

    SelectObject(hdcBack, oldBack);
    DeleteObject(hbmBack);
    DeleteDC(hdcBack);

    DeleteDC(memDC);
    ReleaseDC(hwnd, hdcWindow);

    if (hSrcBmp) DeleteObject(hSrcBmp);
    for (auto &p : particles) if (p.bmp) DeleteObject(p.bmp);
    return 0;
}
