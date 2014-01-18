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

#include "voronoi.h"

#include <string.h>

#include "utils.h"

void vr_diagram_init(vr_diagram_t* v, double w, double h)
{
	v->width  = w;
	v->height = h;

	v->n_edges = 0;
	v->a_edges = 0;
	v->edges   = NULL;

	v->n_vertices = 0;
	v->a_vertices = 0;
	v->vertices   = NULL;

	v->n_regions = 0;
	v->a_regions = 0;
	v->regions   = NULL;

	heap_init(&v->events);
	vr_binbeach_init(&v->front);
	v->sweepline  = 0;
}

void vr_diagram_exit(vr_diagram_t* v)
{
	vr_binbeach_exit(&v->front);

	vr_event_t* e;
	while ((e = heap_remove(&v->events)) != NULL)
		free(e);
	heap_exit(&v->events);

	for (size_t i = 0; i < v->n_regions; i++)
	{
		vr_region_t* r = v->regions[i];
		free(r->edges);
		free(r);
	}
	free(v->regions);

	for (size_t i = 0; i < v->n_edges; i++)
		free(v->edges[i]);
	free(v->edges);

	for (size_t i = 0; i < v->n_vertices; i++)
	{
		vr_vertex_t* p = v->vertices[i];
		free(p->edges);
		free(p);
	}
	free(v->vertices);
}

void vr_diagram_point(vr_diagram_t* v, point_t p)
{
	if (v->n_regions == v->a_regions)
	{
		v->a_regions = v->a_regions == 0 ? 1 : 2*v->a_regions;
		v->regions = CREALLOC(v->regions, vr_region_t*, v->a_regions);
	}
	vr_region_t* r = CALLOC(vr_region_t, 1);
	*r = (vr_region_t){p, 0, NULL};
	v->regions[v->n_regions++] = r;

	vr_event_t* e = CALLOC(vr_event_t, 1);
	*e = (vr_event_t){0, 1, r, NULL, NULL};
	heap_insert(&v->events, p.x, e);
}

void vr_diagram_points(vr_diagram_t* v, size_t n, point_t* p)
{
	for (; n; p++, n--)
		vr_diagram_point(v, *p);
}

static vr_vertex_t* new_vertex(vr_diagram_t* v)
{
	if (v->n_vertices == v->a_vertices)
	{
		v->a_vertices = v->a_vertices == 0 ? 1 : 2*v->a_vertices;
		v->vertices = CREALLOC(v->vertices, vr_vertex_t*, v->a_vertices);
	}
	vr_vertex_t* np = CALLOC(vr_vertex_t, 1);
	*np = (vr_vertex_t) {{0,0}, 0, NULL};
	v->vertices[v->n_vertices++] = np;
	return np;
}
static void push_circle(vr_diagram_t* v, vr_bnode_t* n)
{
	if (n->event != NULL)
		n->event->active = 0;
	n->event = NULL;

	// find previous and next arcs
	vr_bnode_t* pa = vr_bnode_prev(n);
	vr_bnode_t* na = vr_bnode_next(n);

	if (pa == NULL || na == NULL)
		return;

	point_t p;
	double r;
	if (!circle_from3(&p, &r, &pa->r1->p, &n->r1->p, &na->r1->p))
		return;

	vr_event_t* e = CALLOC(vr_event_t, 1);
	e->active = 1;
	e->r = n->r1;

	e->p = new_vertex(v);
	e->p->p = p;

	e->is_circle = 1;
	e->n = n;
	heap_insert(&v->events, e->p->p.x + r, e);
	n->event = e;
}

static void region_addEdge(vr_region_t* a, vr_edge_t* e)
{
	a->edges = CREALLOC(a->edges, vr_edge_t*, a->n_edges+1);
	a->edges[a->n_edges++] = e;
}
static vr_edge_t* new_edge(vr_diagram_t* v, vr_region_t* a, vr_region_t* b)
{
	if (v->n_edges == v->a_edges)
	{
		v->a_edges = v->a_edges == 0 ? 1 : 2*v->a_edges;
		v->edges = CREALLOC(v->edges, vr_edge_t*, v->a_edges);
	}

	vr_edge_t* e = CALLOC(vr_edge_t, 1);
	*e = (vr_edge_t){{NULL, NULL}, a, b};
	if (a != NULL) region_addEdge(a, e);
	if (b != NULL) region_addEdge(b, e);

	v->edges[v->n_edges++] = e;
	return e;
}
char vr_diagram_step(vr_diagram_t* v)
{
	double idx = 0;
	if (v->events.size != 0)
		idx = v->events.tree[0].idx;

	vr_event_t* e = heap_remove(&v->events);
	if (e == NULL)
		return 0;

	if (!e->active)
	{
		free(e);
		return 1;
	}

	v->sweepline = idx;

	if (e->is_circle)
	{
		// current arc
		vr_bnode_t* n = e->n;

		// finish edges at breakpoints
		vr_bnode_t* lb = vr_bnode_right(n);
		vr_bnode_t* rb = vr_bnode_left (n);
		*lb->end = &e->p->p;
		*rb->end = &e->p->p;

		// save previous and next arcs
		vr_bnode_t* pa = vr_bnode_prev(n);
		vr_bnode_t* na = vr_bnode_next(n);

		// remove arc
		n = vr_bnode_remove(n);

		// refresh circle events
		push_circle(v, pa);
		push_circle(v, na);

		// start new edge
		vr_edge_t* f = new_edge(v, pa->r1, na->r1);
		f->s.a = &e->p->p;
		n->end = &f->s.b;
	}
	else
	{
		vr_bnode_t* n = vr_binbeach_breakAt(&v->front, v->sweepline, e->r);

		if (n->left == NULL)
		{
			free(e);
			return 1;
		}

		// insert events
		push_circle(v, n->left);
		push_circle(v, n->right->right);

		// add edge
		vr_edge_t* f = new_edge(v, n->r1, e->r);
		n       ->end = &f->s.a;
		n->right->end = &f->s.b;
	}

	free(e);
	return 1;
}

static void finishEdges(vr_diagram_t* v, vr_bnode_t* n)
{
	if (n->left == NULL)
		return;

	vr_vertex_t* p = new_vertex(v);
	parabola_intersect(&p->p, &n->r1->p, &n->r2->p, v->sweepline);
	*n->end = &p->p;

	finishEdges(v, n->left);
	finishEdges(v, n->right);
}
static char inRect(vr_diagram_t* v, point_t* p)
{
	return
	0 < p->x && p->x < v->width  &&
	0 < p->y && p->y < v->height &&
	1;
}
static void vr_diagram_restrictRegion(vr_diagram_t* v, vr_region_t* r)
{
	point_t corners[4] =
	{
		{       0,        0},
		{       0,v->height},
		{v->width,v->height},
		{v->width,        0},
	};
	segment_t border[4] =
	{
		{&corners[0], &corners[1]},
		{&corners[1], &corners[2]},
		{&corners[2], &corners[3]},
		{&corners[3], &corners[0]},
	};

	// find two jutting edges
	point_t* a = NULL;
	point_t* b = NULL;
	for (ssize_t j = 0; j < (ssize_t) r->n_edges; j++)
	{
		vr_edge_t* e = r->edges[j];
		char ak = inRect(v, e->s.a);
		char bk = inRect(v, e->s.b);

		if (!ak && !bk) // outside edge
		{
			r->n_edges--;
			memmove(r->edges+j, r->edges+j+1, sizeof(vr_edge_t*)*(r->n_edges-j));
			j--;
		}
		else if (ak != bk) // jutting edge
		{
			point_t* p = !ak ? e->s.a : e->s.b;
			if (a == NULL) // first jutting edge
			{
				// TODO
				// BEGIN quickfix
				// this edge might share its end with the other
				// jutting edge; we need to split this end
				vr_vertex_t* np = new_vertex(v);
				np->p = *p;
				p = &np->p;
				if (!ak) e->s.a = p;
				else e->s.b = p;
				// END quickfix

				a = p;
			}
			else // second jutting edge
			{
				b = p;
			}

			// crop the edge
			for (size_t k = 0; k < 4 && !segment_intersect(p, &border[k], &e->s); k++);
		}
	}

	if (a == NULL || b == NULL)
		return;

	point_t p;
	// handle sides
	if (a->x == b->x || a->y == b->y)
	{
		vr_edge_t* e = new_edge(v, r, NULL);
		e->s.a = a;
		e->s.b = b;
		return;
	}
	// handle corners (two sides)
	else if ((a->x == 0 || a->x == v->width) && (b->y == 0 || b->y == v->height))
	{
		p.x = a->x;
		p.y = b->y;
	}
	else if ((a->y == 0 || a->y == v->height) && (b->x == 0 || b->x == v->width))
	{
		p.x = b->x;
		p.y = a->y;
	}
	// should handle three or four sides
	else
		return;

	// apply common corner correction
	vr_vertex_t* np = new_vertex(v);
	np->p = p;
	vr_edge_t* e1 = new_edge(v, r, NULL);
	vr_edge_t* e2 = new_edge(v, r, NULL);
	e1->s.a = a;
	e1->s.b = &np->p;
	e2->s.a = &np->p;
	e2->s.b = b;
}
void vr_diagram_end(vr_diagram_t* v)
{
	while (vr_diagram_step(v));

	v->sweepline += 1000;
	finishEdges(v, v->front.root);

	for (size_t i = 0; i < v->n_regions; i++)
		vr_diagram_restrictRegion(v, v->regions[i]);
}

static void vertex_addEdge(vr_vertex_t* p, vr_edge_t* e)
{
	p->edges = CALLOC(vr_edge_t*, p->n_edges+1);
	p->edges[p->n_edges++] = e;
}
void vr_diagram_fill(vr_diagram_t* v)
{
	for (size_t i = 0; i < v->n_edges; i++)
	{
		vr_edge_t* e = v->edges[i];
		vertex_addEdge((vr_vertex_t*) e->s.a, e);
		vertex_addEdge((vr_vertex_t*) e->s.b, e);
	}
}
