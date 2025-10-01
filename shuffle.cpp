#include <windows.h>
#include <vector>
#include <cstdlib>
#include <ctime>
#include <algorithm>
#include <random>
#include <cmath> // עבור std::fabs

struct Particle {
    HBITMAP bmp;
    int w, h;
    double x, y;
    double targetX, targetY;
    bool alive;
};

int screenW = 0, screenH = 0;
HBITMAP hSrcBmp = nullptr;
std::vector<Particle> particles;

void SetDPIAwareness() {
    HMODULE hUser32 = LoadLibraryA("user32.dll");
    if(hUser32) {
        typedef BOOL(WINAPI* SetProcessDPIAwareFunc)();
        SetProcessDPIAwareFunc pFunc = (SetProcessDPIAwareFunc)GetProcAddress(hUser32, "SetProcessDPIAware");
        if(pFunc) pFunc();
        FreeLibrary(hUser32);
    }
}

HBITMAP CaptureScreen(int &outW, int &outH) {
    HDC hScreen = GetDC(NULL);
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

void InitParticles(HDC hdcSrc, int particleSize = 4) {
    particles.clear();
    srand((unsigned)time(NULL));

    std::vector<POINT> availableTargets;
    for (int y = 0; y < screenH; y += particleSize)
        for (int x = 0; x < screenW; x += particleSize)
            availableTargets.push_back({x, y});

    // שימוש ב-std::shuffle במקום random_shuffle
    std::random_device rd;
    std::mt19937 g(rd());
    std::shuffle(availableTargets.begin(), availableTargets.end(), g);

    int idx = 0;
    for (int y = 0; y < screenH; y += particleSize) {
        for (int x = 0; x < screenW; x += particleSize) {
            Particle p;
            p.w = particleSize; p.h = particleSize;
            p.x = x; p.y = y;
            p.targetX = availableTargets[idx].x;
            p.targetY = availableTargets[idx].y;
            idx++;
            p.alive = true;

            p.bmp = CreateCompatibleBitmap(hdcSrc, particleSize, particleSize);
            HDC partDC = CreateCompatibleDC(hdcSrc);
            HBITMAP old = (HBITMAP)SelectObject(partDC, p.bmp);
            BitBlt(partDC, 0, 0, particleSize, particleSize, hdcSrc, x, y, SRCCOPY);
            SelectObject(partDC, old);
            DeleteDC(partDC);

            particles.push_back(p);
        }
    }
}

LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    if(msg == WM_DESTROY) {
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProc(hwnd, msg, wParam, lParam);
}

int WINAPI WinMain(HINSTANCE hInst, HINSTANCE, LPSTR, int) {
    SetDPIAwareness();
    hSrcBmp = CaptureScreen(screenW, screenH);
    if(!hSrcBmp) return 1;

    const char CLASS_NAME[] = "ScreenParticles";
    WNDCLASSA wc = {};
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInst;
    wc.lpszClassName = CLASS_NAME;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(
        WS_EX_TOPMOST,
        CLASS_NAME, "Screen Particles",
        WS_POPUP | WS_VISIBLE,
        0, 0, screenW, screenH,
        NULL, NULL, hInst, NULL
    );

    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);

    HDC hdcWindow = GetDC(hwnd);
    HDC hdcSrc = CreateCompatibleDC(hdcWindow);
    HBITMAP oldSrc = (HBITMAP)SelectObject(hdcSrc, hSrcBmp);

    InitParticles(hdcSrc, 16);

    HDC hdcBack = CreateCompatibleDC(hdcWindow);
    HBITMAP hbmBack = CreateCompatibleBitmap(hdcWindow, screenW, screenH);
    HBITMAP oldBack = (HBITMAP)SelectObject(hdcBack, hbmBack);
    HDC memDC = CreateCompatibleDC(hdcBack);

    MSG msg;
    ULONGLONG prev = GetTickCount64();
    bool running = true;

    while(running) {
        while(PeekMessage(&msg, NULL, 0, 0, PM_REMOVE)) {
            if(msg.message == WM_QUIT) { running = false; break; }
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }
        if(!running) break;

        ULONGLONG now = GetTickCount64();
        double dt = (now - prev) / 1000.0;
        prev = now;

        RECT rc = {0,0,screenW,screenH};
        FillRect(hdcBack, &rc, (HBRUSH)GetStockObject(BLACK_BRUSH));

        for(auto &p : particles) {
            if(!p.alive) continue;

            double dx = p.targetX - p.x;
            double dy = p.targetY - p.y;

            if(std::fabs(dx) < 0.5 && std::fabs(dy) < 0.5) {
                p.x = p.targetX;
                p.y = p.targetY;
            } else {
                p.x += dx * 0.1;
                p.y += dy * 0.1;
            }

            HBITMAP old = (HBITMAP)SelectObject(memDC, p.bmp);
            BitBlt(hdcBack, (int)p.x, (int)p.y, p.w, p.h, memDC, 0, 0, SRCCOPY);
            SelectObject(memDC, old);
        }

        BitBlt(hdcWindow, 0, 0, screenW, screenH, hdcBack, 0, 0, SRCCOPY);
        Sleep(16);
    }

    SelectObject(hdcSrc, oldSrc);
    DeleteDC(hdcSrc);
    SelectObject(hdcBack, oldBack);
    DeleteObject(hbmBack);
    DeleteDC(hdcBack);
    DeleteDC(memDC);
    ReleaseDC(hwnd, hdcWindow);

    if(hSrcBmp) DeleteObject(hSrcBmp);
    for(auto &p : particles) if(p.bmp) DeleteObject(p.bmp);

    return 0;
}
