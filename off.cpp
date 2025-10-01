//g++ off.cpp -o off.exe -lgdi32 -lmsimg32  -mwindows -std=c++17
#include <windows.h>
#include <string>
#include <cmath>
#include <chrono>
#include <thread>
// ציור עיגול מסתובב כקשת
void DrawRotatingCircle(HDC hdc, int cx, int cy, int r, double angle) {
    HPEN pen = CreatePen(PS_SOLID, 6, RGB(255,255,255));
    HPEN oldPen = (HPEN)SelectObject(hdc, pen);

    // מציירים קשת קטנה לאורך 30 מעלות
    for(double a = angle; a < angle + 0.52; a += 0.05) { // ~30 מעלות
        int x1 = cx + int(r * cos(a));
        int y1 = cy + int(r * sin(a));
        int x2 = cx + int(r * cos(a + 0.05));
        int y2 = cy + int(r * sin(a + 0.05));
        MoveToEx(hdc, x1, y1, NULL);
        LineTo(hdc, x2, y2);
    }

    SelectObject(hdc, oldPen);
    DeleteObject(pen);
}

void SetDPIAwareness() {
    HMODULE hUser32 = LoadLibraryA("user32.dll");
    if (hUser32) {
        typedef BOOL(WINAPI* SetProcessDPIAwareFunc)();
        SetProcessDPIAwareFunc pFunc = (SetProcessDPIAwareFunc)GetProcAddress(hUser32, "SetProcessDPIAware");
        if (pFunc) pFunc();
        FreeLibrary(hUser32);
    }
}

int screenW, screenH;
HBITMAP hSrcBmp = nullptr;

// צילום מסך
HBITMAP CaptureScreen(int &outW, int &outH) {
    SetDPIAwareness();
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

// ציור סצנה על DC נתון
void DrawScene(HDC hdcBack, HBITMAP hBmp, double darkFactor, double angle) {
    HDC memDC = CreateCompatibleDC(hdcBack);
    HBITMAP oldBmp = (HBITMAP)SelectObject(memDC, hBmp);
    BitBlt(hdcBack, 0, 0, screenW, screenH, memDC, 0, 0, SRCCOPY);
    SelectObject(memDC, oldBmp);
    DeleteDC(memDC);

    // שכבת חשיכה
    HBRUSH black = CreateSolidBrush(RGB(0, 0, 0));
    RECT rc = {0,0,screenW,screenH};
    HDC hdcBlend = CreateCompatibleDC(hdcBack);
    HBITMAP hbmBlend = CreateCompatibleBitmap(hdcBack, screenW, screenH);
    HBITMAP oldBlend = (HBITMAP)SelectObject(hdcBlend, hbmBlend);
    FillRect(hdcBlend, &rc, black);

    BLENDFUNCTION bf = {AC_SRC_OVER, 0, (BYTE)(darkFactor * 255), 0};
    AlphaBlend(hdcBack, 0, 0, screenW, screenH, hdcBlend, 0, 0, screenW, screenH, bf);

    SelectObject(hdcBlend, oldBlend);
    DeleteObject(hbmBlend);
    DeleteDC(hdcBlend);
    DeleteObject(black);

    // ציור טקסט
    SetTextColor(hdcBack, RGB(255, 255, 255));
    SetBkMode(hdcBack, TRANSPARENT);
    HFONT hFont = CreateFont(60, 0, 0, 0, FW_BOLD, FALSE, FALSE, FALSE, DEFAULT_CHARSET,
                             OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY,
                             DEFAULT_PITCH | FF_SWISS, "Arial");
    HFONT oldFont = (HFONT)SelectObject(hdcBack, hFont);
    std::string txt = "OFF...";
    SIZE sz;
    GetTextExtentPoint32A(hdcBack, txt.c_str(), txt.size(), &sz);
    int tx = (screenW - sz.cx) / 2;
    int ty = (screenH - sz.cy) / 2;
    TextOutA(hdcBack, tx, ty, txt.c_str(), txt.size());
    SelectObject(hdcBack, oldFont);
    DeleteObject(hFont);

    DrawRotatingCircle(hdcBack, screenW/2, screenH/2 + 100, 30, angle);

}

int WINAPI WinMain(HINSTANCE hInst, HINSTANCE, LPSTR, int) {
    hSrcBmp = CaptureScreen(screenW, screenH);
	ShowCursor(FALSE);
    const char CLASS_NAME[] = "ScreenOff";
    WNDCLASS wc = {};
    wc.lpfnWndProc = DefWindowProc;
    wc.hInstance = hInst;
    wc.lpszClassName = CLASS_NAME;
    RegisterClass(&wc);

    HWND hwnd = CreateWindowEx(WS_EX_TOPMOST, CLASS_NAME, "", WS_POPUP | WS_VISIBLE,
                               0, 0, screenW, screenH, NULL, NULL, hInst, NULL);

    HDC hdcWindow = GetDC(hwnd);

    double angle = 0.0;

    // Double buffer
    HDC hdcBack = CreateCompatibleDC(hdcWindow);
    HBITMAP hbmBack = CreateCompatibleBitmap(hdcWindow, screenW, screenH);
    HBITMAP oldBack = (HBITMAP)SelectObject(hdcBack, hbmBack);

    // חשיכה הדרגתית
    for (int i = 0; i <= 100; i++) {
        double darkFactor = i / 100.0;
        DrawScene(hdcBack, hSrcBmp, darkFactor, angle);
        BitBlt(hdcWindow, 0, 0, screenW, screenH, hdcBack, 0, 0, SRCCOPY);
        angle += 0.2;
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
    }

    // טקסט ומעגל למשך זמן קצר
    for (int i = 0; i < 50; i++) {
        DrawScene(hdcBack, hSrcBmp, 1.0, angle);
        BitBlt(hdcWindow, 0, 0, screenW, screenH, hdcBack, 0, 0, SRCCOPY);
        angle += 0.2;
        std::this_thread::sleep_for(std::chrono::milliseconds(30));
    }

    // שמירה על המסך שחור בלי הבהוב
    RECT rcBlack = {0, 0, screenW, screenH};
FillRect(hdcBack, &rcBlack, (HBRUSH)GetStockObject(BLACK_BRUSH));
    BitBlt(hdcWindow, 0, 0, screenW, screenH, hdcBack, 0, 0, SRCCOPY);
	
    MSG msg;
	while (true) {
		RECT rcBlack = {0, 0, screenW, screenH};
		FillRect(hdcBack, &rcBlack, (HBRUSH)GetStockObject(BLACK_BRUSH));
		BitBlt(hdcWindow, 0, 0, screenW, screenH, hdcBack, 0, 0, SRCCOPY);
		if (PeekMessage(&msg, NULL, 0, 0, PM_REMOVE)) {
			if (msg.message == WM_QUIT) break;
			if (msg.message == WM_KEYDOWN && msg.wParam == VK_ESCAPE) break;
			TranslateMessage(&msg);
			DispatchMessage(&msg);
		}
		std::this_thread::sleep_for(std::chrono::milliseconds(10));
	}
    SelectObject(hdcBack, oldBack);
    DeleteObject(hbmBack);
    DeleteDC(hdcBack);
    ReleaseDC(hwnd, hdcWindow);
    DeleteObject(hSrcBmp);
	
    return 0;
}
