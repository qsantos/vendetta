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

#include "lloyd.h"

#include <stdlib.h>
#include <math.h>

static double heading(point_t p)
{
	double dir;
	if (p.x > 0)
	{
		dir = atan(p.y / p.x);
		if (p.y < 0)
			dir += 2 * M_PI;
	}
	else if (p.x < 0)
	{
		dir = atan(p.y / p.x) + M_PI;
	}
	else // p.x == 0
	{
		dir = M_PI / 2;
		if (p.y < 0)
			dir += M_PI;
	}
	return dir;
}
static int poly_vert(const void* a, const void* b, void* arg)
{
	const point_t* pa = (const point_t*) a;
	const point_t* pb = (const point_t*) b;
	point_t* c = (point_t*) arg;
	double da = heading((point_t){pa->x-c->x, pa->y-c->y});
	double db = heading((point_t){pb->x-c->x, pb->y-c->y});
	return da < db ? -1 : da > db ? +1 : 0;
}
void vr_region_points(point_t* dst, vr_region_t* r)
{
	// gather vertices (twice)
	point_t tmp[2*r->n_edges];
	for (size_t j = 0; j < r->n_edges; j++)
	{
		segment_t* s = &r->edges[j]->s;
		tmp[2*j]   = *s->a;
		tmp[2*j+1] = *s->b;
	}

	// compute mean point (inside polygon)
	point_t mean = {0,0};
	for (size_t j = 0; j < 2*r->n_edges; j++)
	{
		mean.x += tmp[j].x;
		mean.y += tmp[j].y;
	}
	mean.x /= 2*r->n_edges;
	mean.y /= 2*r->n_edges;

	// order vertices
	qsort_r(tmp, 2*r->n_edges, sizeof(point_t), poly_vert, &mean);

	// filter out multiple points
	for (size_t j = 0; j < r->n_edges; j++)
		dst[j] = tmp[2*j];
}

void vr_lloyd_relaxation(vr_diagram_t* v)
{
	vr_diagram_end(v);

	point_t npoints[v->n_regions];
	size_t k = 0;
	for (size_t i = 0; i < v->n_regions; i++)
	{
		vr_region_t* r = v->regions[i];

		// gather vertices
		point_t vertices[r->n_edges];
		vr_region_points(vertices, r);

		// compute centroid
		point_t c = point_centroid(r->n_edges, vertices);

		if (0 <= c.x && c.x <= v->width && 0 <= c.y && c.y <= v->height)
		{
			npoints[k].x = c.x;
			npoints[k].y = c.y;
			k++;
		}
	}
	double w = v->width;
	double h = v->height;
	vr_diagram_exit(v);
	vr_diagram_init(v, w, h);
	vr_diagram_points(v, k, npoints);
}
