// shatter_triangles_smooth.cpp
// Compile:
// g++ shatter_triangles_smooth.cpp -o shatter.exe -lgdi32 -luser32 -mwindows -std=c++17

#include <windows.h>
#include <vector>
#include <cstdlib>
#include <ctime>
#include <cmath>
#include <algorithm>

struct Tri {
    POINT origP[3];
    int minX, minY;
    int w, h;
    HBITMAP bmpPart;
    double cx0, cy0;
    double cx, cy;
    double vx, vy;
    double angle;
    double omega;
};

std::vector<Tri> tris;
HBITMAP hBmpGlobal = nullptr;
int screenW = 0, screenH = 0;
bool started = false;

void SetDPIAwareness() {
    HMODULE hUser32 = LoadLibrary("user32.dll");
    if(hUser32) {
        typedef BOOL(WINAPI* SetProcessDPIAwareFunc)();
        SetProcessDPIAwareFunc pFunc = (SetProcessDPIAwareFunc)GetProcAddress(hUser32, "SetProcessDPIAware");
        if(pFunc) pFunc();
        FreeLibrary(hUser32);
    }
}

double randf(double a, double b){ return a + (b-a) * (rand() / (RAND_MAX + 1.0)); }

HBITMAP CaptureScreenBitmap(int &outW, int &outH) {
    HDC hScreen = GetDC(NULL);
    outW = GetDeviceCaps(hScreen, HORZRES);
    outH = GetDeviceCaps(hScreen, VERTRES);

    HDC mem = CreateCompatibleDC(hScreen);
    HBITMAP bmp = CreateCompatibleBitmap(hScreen, outW, outH);
    HBITMAP old = (HBITMAP)SelectObject(mem, bmp);

    BitBlt(mem, 0, 0, outW, outH, hScreen, 0, 0, SRCCOPY | CAPTUREBLT);

    SelectObject(mem, old);
    DeleteDC(mem);
    ReleaseDC(NULL, hScreen);
    return bmp;
}

HBITMAP CropRect(HDC srcDC, int x, int y, int w, int h) {
    if (w <= 0 || h <= 0) return nullptr;
    HDC mem = CreateCompatibleDC(srcDC);
    HBITMAP bmp = CreateCompatibleBitmap(srcDC, w, h);
    HBITMAP old = (HBITMAP)SelectObject(mem, bmp);
    BitBlt(mem, 0, 0, w, h, srcDC, x, y, SRCCOPY);
    SelectObject(mem, old);
    DeleteDC(mem);
    return bmp;
}

void BuildTriangulation(HBITMAP fullBmp, int gridX, int gridY) {
    for (auto &t : tris) if (t.bmpPart) DeleteObject(t.bmpPart);
    tris.clear();

    HDC srcDC = CreateCompatibleDC(NULL);
    HBITMAP old = (HBITMAP)SelectObject(srcDC, fullBmp);

    screenW = GetSystemMetrics(SM_CXSCREEN);
    screenH = GetSystemMetrics(SM_CYSCREEN);

    int cellW = screenW / gridX;
    int cellH = screenH / gridY;

// מספר נקודות אקראיות (ללא תלות ב-grid)
int numPoints = 50;  // תוכל לשנות בהתאם לגודל המשולשים הרצוי
std::vector<POINT> pts;
pts.reserve(numPoints + 7);

// הוספת פינות המסך
pts.push_back({0, 0});
pts.push_back({screenW-1, 0});
pts.push_back({0, screenH-1});
pts.push_back({screenW-1, screenH-1});
// יצירת נקודות אקראיות
for (int i = 0; i < numPoints; ++i) {
    pts.push_back({ LONG(randf(0.0, double(screenW))), LONG(randf(0.0, double(screenH))) });
}



// super-triangle גדול מאוד כדי לכסות את כל המסך
const int M = 10000000;
pts.push_back({-M, -M});
pts.push_back({3*M + screenW, -M});
pts.push_back({-M, 3*M + screenH});

// מבנה אינדקסי למשולשים
struct TriIdx { int a,b,c; };
std::vector<TriIdx> triList;
triList.push_back({int(pts.size()-3), int(pts.size()-2), int(pts.size()-1)}); // התחלת טריאנגולציה עם ה-super-triangle

// פונקציה לבדוק אם נקודה בתוך המעגל ההיקפי של משולש
auto circumcircleContains = [&](const TriIdx &t, const POINT &p)->bool{
    double ax = pts[t.a].x, ay = pts[t.a].y;
    double bx = pts[t.b].x, by = pts[t.b].y;
    double cx = pts[t.c].x, cy = pts[t.c].y;
    double A = bx - ax, B = by - ay, C = cx - ax, D = cy - ay;
    double E = A*(ax+bx) + B*(ay+by);
    double F = C*(ax+cx) + D*(ay+cy);
    double G = 2.0*(A*(cy-by)-B*(cx-bx));
    if (fabs(G)<1e-12) return false;
    double cxCirc = (D*E - B*F)/G;
    double cyCirc = (A*F - C*E)/G;
    double dx = ax - cxCirc, dy = ay - cyCirc;
    double r2 = dx*dx + dy*dy;
    double dxp = p.x - cxCirc, dyp = p.y - cyCirc;
    return (dxp*dxp + dyp*dyp) <= r2 + 1e-6;
};

// Bowyer–Watson
for (int i=0;i<numPoints;i++) {
    POINT p = pts[i];
    std::vector<int> bad;
    for (int ti=0; ti<(int)triList.size(); ti++)
        if (circumcircleContains(triList[ti], p)) bad.push_back(ti);

    struct Edge { int u,v; };
    std::vector<Edge> polyEdges;
    auto addEdge = [&](int a,int b){
        for(size_t ei=0; ei<polyEdges.size(); ei++){
            if(polyEdges[ei].u==b && polyEdges[ei].v==a){
                polyEdges.erase(polyEdges.begin()+ei);
                return;
            }
        }
        polyEdges.push_back({a,b});
    };

    for(int idx : bad){
        TriIdx &t = triList[idx];
        addEdge(t.a,t.b); addEdge(t.b,t.c); addEdge(t.c,t.a);
    }

    std::sort(bad.begin(), bad.end(), std::greater<int>());
    for(int idx: bad) triList.erase(triList.begin()+idx);

    for(auto &e: polyEdges)
        triList.push_back({e.u,e.v,i});
}

// סינון משולשים שמכילים ורטקס של super-triangle
std::vector<TriIdx> finalTris;
for(auto &t: triList)
    if(t.a<numPoints && t.b<numPoints && t.c<numPoints)
        finalTris.push_back(t);

// המרה ל-Tri
for(auto &ft: finalTris){
    Tri T{};
    T.origP[0] = pts[ft.a]; T.origP[1] = pts[ft.b]; T.origP[2] = pts[ft.c];
    int minX = std::min({T.origP[0].x, T.origP[1].x, T.origP[2].x});
    int minY = std::min({T.origP[0].y, T.origP[1].y, T.origP[2].y});
    int maxX = std::max({T.origP[0].x, T.origP[1].x, T.origP[2].x});
    int maxY = std::max({T.origP[0].y, T.origP[1].y, T.origP[2].y});
    T.minX = minX; T.minY = minY;
    T.w = maxX-minX+1; T.h = maxY-minY+1;
    T.bmpPart = CropRect(srcDC, T.minX, T.minY, T.w, T.h);
    T.cx0 = (T.origP[0].x + T.origP[1].x + T.origP[2].x)/3.0;
    T.cy0 = (T.origP[0].y + T.origP[1].y + T.origP[2].y)/3.0;
    T.cx=T.cx0; T.cy=T.cy0; T.vx=T.vy=T.angle=T.omega=0;
    tris.push_back(T);
}

    SelectObject(srcDC, old);
    DeleteDC(srcDC);
}

void rotate(double rx, double ry, double angle, double &ox, double &oy) {
    double c = cos(angle), s = sin(angle);
    ox = rx * c - ry * s;
    oy = rx * s + ry * c;
}

void RenderToDC(HDC hdc) {
    RECT rc = { 0, 0, screenW, screenH };
FillRect(hdc, &rc, (HBRUSH)GetStockObject(BLACK_BRUSH));

    HDC mem = CreateCompatibleDC(hdc);

    for (auto &t : tris) {
        if (!t.bmpPart) continue;
        HBITMAP old = (HBITMAP)SelectObject(mem, t.bmpPart);

        if (!started) {
            POINT poly[3] = { t.origP[0], t.origP[1], t.origP[2] };
            HRGN r = CreatePolygonRgn(poly, 3, WINDING);
            SelectClipRgn(hdc, r);
            BitBlt(hdc, t.minX, t.minY, t.w, t.h, mem, 0, 0, SRCCOPY);
            SelectClipRgn(hdc, NULL);
            DeleteObject(r);
        } else {
            POINT destTri[3];
            for (int i=0;i<3;i++){
                double rx = t.origP[i].x - t.cx0;
                double ry = t.origP[i].y - t.cy0;
                double ox, oy;
                rotate(rx, ry, t.angle, ox, oy);
                destTri[i].x = LONG(t.cx + ox);
                destTri[i].y = LONG(t.cy + oy);
            }
            HRGN r = CreatePolygonRgn(destTri, 3, WINDING);
            SelectClipRgn(hdc, r);

            double halfW = t.w/2.0;
            double halfH = t.h/2.0;
            double ox1, oy1, ox2, oy2, ox3, oy3;
            rotate(-halfW, -halfH, t.angle, ox1, oy1);
            rotate( halfW, -halfH, t.angle, ox2, oy2);
            rotate(-halfW,  halfH, t.angle, ox3, oy3);

            POINT plg[3] = {
                {LONG(t.cx + ox1), LONG(t.cy + oy1)},
                {LONG(t.cx + ox2), LONG(t.cy + oy2)},
                {LONG(t.cx + ox3), LONG(t.cy + oy3)}
            };
            PlgBlt(hdc, plg, mem, 0, 0, t.w, t.h, NULL, 0, 0);
            SelectClipRgn(hdc, NULL);
            DeleteObject(r);
        }

        SelectObject(mem, old);
    }

    DeleteDC(mem);
}

void Render(HWND hwnd) {
    PAINTSTRUCT ps;
    HDC hdc = BeginPaint(hwnd, &ps);

    HDC memDC = CreateCompatibleDC(hdc);
    HBITMAP hBack = CreateCompatibleBitmap(hdc, screenW, screenH);
    HBITMAP oldBmp = (HBITMAP)SelectObject(memDC, hBack);

    RenderToDC(memDC);

    BitBlt(hdc, 0, 0, screenW, screenH, memDC, 0, 0, SRCCOPY);

    SelectObject(memDC, oldBmp);
    DeleteObject(hBack);
    DeleteDC(memDC);
    EndPaint(hwnd, &ps);
}

void StepPhysics(double dt) {
    const double gravity = 300.0;
    for (auto &t : tris) {
        t.vy += gravity*dt;
        t.cx += t.vx*dt;
        t.cy += t.vy*dt;
        t.angle += t.omega*dt;
    }
}

void StartAnimation() {
	ShowCursor(FALSE);
    for (auto &t : tris) {
        double dirx = t.cx0 - (screenW/2.0);
        double diry = t.cy0 - (screenH/2.0);
        double dist = sqrt(dirx*dirx + diry*diry) + 1.0;
        dirx /= dist; diry /= dist;
        double speed = randf(30.0, 140.0);
        t.vx = dirx*speed + randf(-40.0,40.0);
        t.vy = randf(-20.0,20.0);
        t.omega = randf(-3.0,3.0);
        t.cx = t.cx0; t.cy = t.cy0;
    }
    started = true;
}

LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam) {
    switch(msg){
        case WM_PAINT: Render(hwnd); return 0;
        case WM_LBUTTONDOWN: if(!started) StartAnimation(); return 0;
        case WM_KEYDOWN: if(wParam == VK_ESCAPE) PostQuitMessage(0); return 0;
        case WM_DESTROY: PostQuitMessage(0); return 0;
        default: return DefWindowProc(hwnd, msg, wParam, lParam);
    }
}

int WINAPI WinMain(HINSTANCE hInst, HINSTANCE, LPSTR, int) {
    srand((unsigned)time(NULL));
    SetDPIAwareness();

    hBmpGlobal = CaptureScreenBitmap(screenW, screenH);
    if(!hBmpGlobal){ MessageBoxA(NULL,"Failed to capture screen.","Error",MB_ICONERROR); return 1; }

    const int GRID_X = 5, GRID_Y = 5;
    BuildTriangulation(hBmpGlobal, GRID_X, GRID_Y);

    const char CLASS_NAME[] = "ShatterTriWindow";
    WNDCLASSA wc = {};
    wc.lpfnWndProc = WndProc;
    wc.hInstance = hInst;
    wc.lpszClassName = CLASS_NAME;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.hbrBackground = NULL;
    RegisterClassA(&wc);

    HWND hwnd = CreateWindowExA(WS_EX_TOPMOST, CLASS_NAME, "", WS_POPUP | WS_VISIBLE,
                                0, 0, screenW, screenH, nullptr, nullptr, hInst, nullptr);
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
	
    LARGE_INTEGER freq, lastCounter;
    QueryPerformanceFrequency(&freq);
    QueryPerformanceCounter(&lastCounter);

    MSG msg;
    while(true){
        while(PeekMessage(&msg, nullptr,0,0, PM_REMOVE)){
            if(msg.message==WM_QUIT) goto finish;
            TranslateMessage(&msg);
            DispatchMessage(&msg);
        }

        LARGE_INTEGER nowCounter;
        QueryPerformanceCounter(&nowCounter);
        double dt = double(nowCounter.QuadPart - lastCounter.QuadPart) / freq.QuadPart;
        lastCounter = nowCounter;
        if(dt > 0.033) dt=0.033;

        if(started){
            StepPhysics(dt);
            InvalidateRect(hwnd, NULL, FALSE);
        }
        Sleep(1);
    }

finish:
    for(auto &t : tris) if(t.bmpPart) DeleteObject(t.bmpPart);
    if(hBmpGlobal) DeleteObject(hBmpGlobal);
    return 0;
}
