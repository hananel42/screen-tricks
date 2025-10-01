#include <windows.h>
#include <math.h>

LPCSTR CLASS_NAME = "WaveWindow";

int screenW, screenH;
HBITMAP screenBmp;
double phase = 0.0;
void SetDPIAwareness() {
    HMODULE hUser32 = LoadLibrary("user32.dll");
    if(hUser32) {
        typedef BOOL(WINAPI* SetProcessDPIAwareFunc)();
        SetProcessDPIAwareFunc pFunc = (SetProcessDPIAwareFunc)GetProcAddress(hUser32, "SetProcessDPIAware");
        if(pFunc) pFunc();
        FreeLibrary(hUser32);
    }
}
HBITMAP CaptureScreen(HWND hwnd) {
    HDC hdcScreen = GetDC(NULL);
    HDC hdcMem = CreateCompatibleDC(hdcScreen);
    HBITMAP hbmScreen = CreateCompatibleBitmap(hdcScreen, screenW, screenH);
    SelectObject(hdcMem, hbmScreen);
    BitBlt(hdcMem, 0, 0, screenW, screenH, hdcScreen, 0, 0, SRCCOPY);
    DeleteDC(hdcMem);
    ReleaseDC(NULL, hdcScreen);
    return hbmScreen;
}

void DrawWaveEffect(HDC hdc, HBITMAP bmp, int w, int h, double phase) {
    HDC memDC = CreateCompatibleDC(hdc);
    HBITMAP oldBmp = (HBITMAP)SelectObject(memDC, bmp);

    for (int y = 0; y < h; y++) {
        int offset = int(50 * sin((y / 100.0) + phase));
        BitBlt(hdc,
               offset, y,     // יעד
               w, 1,          // שורה אחת
               memDC,
               0, y,          // מקור
               SRCCOPY);
    }

    SelectObject(memDC, oldBmp);
    DeleteDC(memDC);
}

LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch (msg) {
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProc(hwnd, msg, wParam, lParam);
}

int WINAPI WinMain(HINSTANCE hInst, HINSTANCE, LPSTR, int) {
	SetDPIAwareness();
	screenW = GetSystemMetrics(SM_CXSCREEN);
    screenH = GetSystemMetrics(SM_CYSCREEN);

    // הגדרת חלון
    WNDCLASSA wc = {};
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInst;
    wc.lpszClassName = CLASS_NAME;
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(
        WS_EX_TOPMOST,
        CLASS_NAME, "",
        WS_POPUP | WS_VISIBLE,
        0, 0, screenW, screenH,
        NULL, NULL, hInst, NULL
    );

    ShowCursor(FALSE);

    // צילום מסך ראשוני
    
	screenBmp = CaptureScreen(hwnd);
    // לולאת ציור
    MSG msg;
    HDC hdc = GetDC(hwnd);
    while (true) {
        while (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_QUIT) {
                ReleaseDC(hwnd, hdc);
                return 0;
            }
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }

        // ציור אפקט הגל
        DrawWaveEffect(hdc, screenBmp, screenW, screenH, phase);

        phase += 0.07; // מהירות הגל
        Sleep(16);     // ~60FPS
    }

    return 0;
}
