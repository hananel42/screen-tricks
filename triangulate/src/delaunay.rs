#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

// מימוש השוואה בסיסית בין נקודות לצורך האלגוריתם
impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        (self.x - other.x).abs() < 0.0001 && (self.y - other.y).abs() < 0.0001
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    pub p1: Point,
    pub p2: Point,
    pub p3: Point,
}

impl Triangle {
    /// יוצר משולש חדש ומבטיח שהקודקודים שלו מסודרים נגד כיוון השעון (CCW)
    pub fn new(p1: Point, p2: Point, p3: Point) -> Self {
        // חישוב מכפלה וקטורית (Cross Product) כדי לבדוק כיוון
        let cross_product = (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x);

        if cross_product < 0.0 {
            // אם זה עם כיוון השעון, הופכים שתי נקודות כדי לתקן ל-CCW
            Triangle { p1, p2: p3, p3: p2 }
        } else {
            Triangle { p1, p2, p3 }
        }
    }

    /// בדיקת מעגל חוסם באמצעות הטלה לפרבולואיד תלת-ממדי (הטריק המהיר)
    pub fn in_circumcircle(&self, p: Point) -> bool {
        let adx = self.p1.x - p.x;
        let ady = self.p1.y - p.y;
        let bdx = self.p2.x - p.x;
        let bdy = self.p2.y - p.y;
        let cdx = self.p3.x - p.x;
        let cdy = self.p3.y - p.y;

        let abdet = adx * bdy - bdx * ady;
        let bcdet = bdx * cdy - cdx * bdy;
        let cadet = cdx * ady - adx * cdy;

        let alift = adx * adx + ady * ady;
        let blift = bdx * bdx + bdy * bdy;
        let clift = cdx * cdx + cdy * cdy;

        let det = alift * bcdet + blift * cadet + clift * abdet;

        // גדול מאפס אומר שהנקודה בתוך המעגל החוסם
        det > 0.001
    }
}

/// פונקציית הליבה: מקבלת מערך נקודות ומחזירה רשת משולשים אופטימלית
pub fn triangulate(points: &[Point], screen_width: f32, screen_height: f32) -> Vec<Triangle> {
    let mut triangulation = Vec::new();

    // 1. יצירת משולש-על (Super-Triangle) שמכיל את כל גבולות המסך בפער עצום
    let super_p1 = Point {
        x: -screen_width * 10.0,
        y: -screen_height * 10.0,
    };
    let super_p2 = Point {
        x: screen_width * 10.0,
        y: -screen_height * 10.0,
    };
    let super_p3 = Point {
        x: screen_width * 0.5,
        y: screen_height * 10.0,
    };

    triangulation.push(Triangle::new(super_p1, super_p2, super_p3));

    // 2. הוספת נקודות אחת אחת ובנייה מחדש של הרשת
    for &point in points {
        let mut bad_triangles = Vec::new();

        // שלב א': מציאת כל המשולשים שהנקודה החדשה מפרה את תנאי דלוני שלהם
        for &triangle in &triangulation {
            if triangle.in_circumcircle(point) {
                bad_triangles.push(triangle);
            }
        }

        // שלב ב': מציאת קווי המתאר (הצלעות החיצוניות) של החור שנוצר
        let mut polygon = Vec::new();

        for &t in &bad_triangles {
            let edges = [(t.p1, t.p2), (t.p2, t.p3), (t.p3, t.p1)];

            for edge in edges {
                let mut is_shared = false;
                for &other_t in &bad_triangles {
                    if other_t == t {
                        continue;
                    }

                    let other_edges = [
                        (other_t.p1, other_t.p2),
                        (other_t.p2, other_t.p3),
                        (other_t.p3, other_t.p1),
                    ];
                    for other_edge in other_edges {
                        if (edge.0 == other_edge.0 && edge.1 == other_edge.1)
                            || (edge.0 == other_edge.1 && edge.1 == other_edge.0)
                        {
                            is_shared = true;
                            break;
                        }
                    }
                    if is_shared {
                        break;
                    }
                }
                // אם הצלע לא משותפת עם אף משולש "רע" אחר, היא חלק מקו המתאר החיצוני
                if !is_shared {
                    polygon.push(edge);
                }
            }
        }

        // שלב ג': מחיקת המשולשים הגרועים מהרשת הנוכחית
        triangulation.retain(|t| !bad_triangles.contains(t));

        // שלב ד': יצירת משולשים חדשים מהנקודה לכל צלעות קו המתאר
        for edge in polygon {
            triangulation.push(Triangle::new(edge.0, edge.1, point));
        }
    }

    // 3. ניקוי סופי: הסרת כל המשולשים שנוגעים בקודקודים של משולש-העל
    triangulation.retain(|t| {
        !(t.p1 == super_p1
            || t.p2 == super_p1
            || t.p3 == super_p1
            || t.p1 == super_p2
            || t.p2 == super_p2
            || t.p3 == super_p2
            || t.p1 == super_p3
            || t.p2 == super_p3
            || t.p3 == super_p3)
    });

    triangulation
}
