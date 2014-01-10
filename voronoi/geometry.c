/*\
 *  Voronoi diagram by Fortune's algorithm
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "geometry.h"

#include <math.h>

point_t point_minus(point_t a, point_t b)
{
	return (point_t){a.x-b.x,a.y-b.y};
}

double point_cross(point_t a, point_t b)
{
	return a.x*b.y - a.y*b.x;
}

point_t point_centroid(int n, point_t* pts)
{
	point_t ret = {0,0};
	double A = 0;
	for (int i=0, j=n-1; i < n; j=i++)
	{
		point_t p = pts[j];
		point_t q = pts[i];
		double f = p.x*q.y - q.x*p.y;
		ret.x += (p.x+q.x)*f;
		ret.y += (p.y+q.y)*f;
		A += f;
	}
	A /= 2;
	ret.x /= 6*A;
	ret.y /= 6*A;
	return ret;
}

/*
Parabolas are defined as being the set of points equidistant from a
vertex (focus) and a line (directrix). It is exactly what we need for
our beachline where the directrix is the sweepline and the focus is the
considered site.

Considering focus f and directrix of equation x=p, the parabola is
then defined as |x-p| = sqrt((x-f.x)^2+(y-f.y)^2) (using Euclidean
distance). By squaring, we get the condition:

p^2 - 2px = f.x^2 - 2x*f.x + (y-f.y)^2
or
2x(f.x-p) = f.x^2 - p^2 + (y.fy)^2

Thus:
{
2x(f1.x-p) = f1.x^2 - p^2 + (y-f1.y)^2
2x(f2.x-p) = f2.x^2 - p^2 + (y-f2.y)^2
}

The first line gets us:
2x = (f1.x^2 - p^2 + (y-f1.y)^2) / (f1.x-p)

Replacing 2x in the second one, we infer that:

(f1.x^2 - p^2 + (y-f1.y)^2) r = f2.x^2 - p^2 + (y-f2.y)^2

For convenience, let:
r = (f2.x-p)/(f1.x-p)
d1 = f1.x^2 - p^2 + f1.y^2
d2 = f2.x^2 - p^2 + f2.y^2

We now have an equation in y:
(y-f1.y)^2 r - (y-f2.y)^2 = f1.x^2-p^2 - (f2.x^2-p^2)*r
or
y^2 * (r-1) + y * 2(f2.y-f1.y*r) + (d1*r-d2) = 0

Hence, with:
a = r-1
b = 2(f2.y-f1.y*r)
c = d1*r+d2
delta = b^2 - 4ac

We have: y = (-b +- sqrt(delta)) / (2a)
*/
char parabola_intersect(point_t* dst, const point_t* f1, const point_t* f2, double p)
{
	const point_t* f = f1;

	if (f1->x == f2->x)
	{
		dst->y = (f1->y+f2->y)/2;
	}
	else if (f1->x == p)
	{
		dst->y = f1->y;
		f = f2;
	}
	else
	{
		double r = (f2->x-p)/(f1->x-p);
		double d1 = f1->x*f1->x - p*p + f1->y*f1->y;
		double d2 = f2->x*f2->x - p*p + f2->y*f2->y;

		double a = r-1;
		double b = 2*(f2->y - f1->y*r);
		double c = d1*r - d2;
		double delta = b*b - 4*a*c;

		if (delta < 0)
			return 0;
		dst->y = (-b + sqrt(delta)) / (2*a);
	}

	double t = dst->y-f->y;
	dst->x = (f->x*f->x - p*p + t*t) / (2*(f->x-p));
	return 1;
}

/*
Consider circle of center (x,y) and radius r. We are given three points
p1, p2 and p3 and want to find back the center an the radius. We have:

(p1.x-x)^2 + (p1.y-y)^2 = r^2
(p2.x-x)^2 + (p2.y-y)^2 = r^2
(p3.x-x)^2 + (p3.y-y)^2 = r^2

According to http://paulbourke.net/geometry/circlesphere/, it solves to:

a = (p2.y-p1.y) / (p2.x-p1.x)
b = (p3.y-p1.y) / (p3.x-p1.x)

x = (ab(p1.y-p3.y) + b(p1.x+p2.x) - a(p2.x+p3.x)) / (2*(b-a))
y = -(x - (p1.x+p2.x)/2)/a + (p1.y+p2.y)/2

	double a = (p2->y-p1->y) / (p2->x-p1->x);
	double b = (p3->y-p2->y) / (p3->x-p2->x);
	if (a == b)
		return 0;

	double x = (a*b*(p1->y-p3->y) + b*(p1->x+p2->x) - a*(p2->x+p3->x)) / (2*(b-a));
	double y = -(x - (p1->x+p2->x)/2)/a + (p1->y+p2->y)/2;

	double dx = p1->x - x;
	double dy = p1->y - y;
	*r = sqrt(dx*dx + dy*dy);

	c->x = x;
	c->y = y;

	return 1;
not working
*/

// from http://www.cs.hmc.edu/~mbrubeck/voronoi.html
char circle_from3(point_t* c, double* r, const point_t* p1, const point_t* p2, const point_t* p3)
{
	// Check that bc is a "right turn" from ap2->
	if ((p2->x-p1->x)*(p3->y-p1->y) - (p3->x-p1->x)*(p2->y-p1->y) > 0)
		return 0;

	// Algorithm from O'Rourke 2ed p. 189.
	double A = p2->x - p1->x,  B = p2->y - p1->y,
	       C = p3->x - p1->x,  D = p3->y - p1->y,
	       E = A*(p1->x+p2->x) + B*(p1->y+p2->y),
	       F = C*(p1->x+p3->x) + D*(p1->y+p3->y),
	       G = 2*(A*(p3->y-p2->y) - B*(p3->x-p2->x));

	if (G == 0)  // Points are co-linear.
		return 0;

	// Point o is the center of the circle.
	c->x = (D*E-B*F)/G;
	c->y = (A*F-C*E)/G;

	// o.x plus radius equals max x coordinate.
	double dx = p1->x - c->x;
	double dy = p1->y - c->y;
	*r = sqrt(dx*dx+dy*dy);
	return 1;
}

// http://stackoverflow.com/questions/563198/how-do-you-detect-where-two-line-segments-intersect/565282#565282
char segment_intersect(point_t* dst, const segment_t* a, const segment_t* b)
{
	point_t p = *a->a;
	point_t r = point_minus(*a->b,p);
	point_t q = *b->a;
	point_t s = point_minus(*b->b,q);

	double y = point_cross(r, s);
	if (y != 0)
	{
		point_t m = point_minus(q, p);
		double t = point_cross(m, s) / y;
		double u = point_cross(m, r) / y;
		if (0 <= t && t <= 1 && 0 <= u && u <= 1)
		{
			dst->x = p.x + t*r.x;
			dst->y = p.y + t*r.y;
			return 1;
		}
	}
	return 0;
}
