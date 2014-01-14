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

#include "binbeach.h"

#include "utils.h"
#include "voronoi.h"

void vr_binbeach_init(vr_binbeach_t* b)
{
	b->root = NULL;
}

static void exit_aux(vr_bnode_t* n)
{
	if (n == NULL)
		return;

	exit_aux(n->left);
	exit_aux(n->right);
	free(n);
}
void vr_binbeach_exit(vr_binbeach_t* b)
{
	exit_aux(b->root);
}

vr_bnode_t* vr_binbeach_breakAt(vr_binbeach_t* b, double sweep, struct vr_region* r)
{
	if (b->root == NULL)
	{
		vr_bnode_t* n = CALLOC(vr_bnode_t, 1);
		*n = (vr_bnode_t){r, NULL, NULL, NULL, NULL, NULL, NULL};
		b->root = n;
		return n;
	}

	// find the intersecting arc
	double y = r->p.y;
	vr_bnode_t* n = b->root;
	while (n->r2 != NULL)
	{
		point_t p;
		parabola_intersect(&p, &n->r1->p, &n->r2->p, sweep);
		if (y < p.y)
			n = n->left;
		else
			n = n->right;
	}

	// left leaf (original region)
	vr_bnode_t* ll = CALLOC(vr_bnode_t, 1);
	*ll = (vr_bnode_t){n->r1, NULL, NULL, NULL, n, NULL, n->event};
	n->left = ll;

	// new internal node
	vr_bnode_t* ni = CALLOC(vr_bnode_t, 1);

	// middle leaf (new region)
	vr_bnode_t* ml = CALLOC(vr_bnode_t, 1);
	*ml = (vr_bnode_t){r, NULL, NULL, NULL, ni, NULL, NULL};

	// right leaf (original region)
	vr_bnode_t* rl = CALLOC(vr_bnode_t, 1);
	*rl = (vr_bnode_t){n->r1, NULL, NULL, NULL, ni, NULL, n->event};

	// filling new internal node
	*ni = (vr_bnode_t){r, n->r1, ml, rl, n, NULL, NULL};

	// filling old leaf node (now internal)
	n->r2    = r;
	n->left  = ll;
	n->right = ni;

	return n;
}

vr_bnode_t* vr_bnode_left(vr_bnode_t* n)
{
	while (n->parent != NULL && n != n->parent->left)
		n = n->parent;
	return n->parent;
}

vr_bnode_t* vr_bnode_right(vr_bnode_t* n)
{
	while (n->parent != NULL && n != n->parent->right)
		n = n->parent;
	return n->parent;
}

vr_bnode_t* vr_bnode_prev(vr_bnode_t* n)
{
	n = vr_bnode_right(n);
	if (n == NULL)
		return NULL;
	n = n->left;
	while (n->right != NULL)
		n = n->right;
	return n;
}

vr_bnode_t* vr_bnode_next(vr_bnode_t* n)
{
	n = vr_bnode_left(n);
	if (n == NULL)
		return NULL;
	n = n->right;
	while (n->left != NULL)
		n = n->left;
	return n;
}

vr_bnode_t* vr_bnode_remove(vr_bnode_t* n)
{
	vr_bnode_t* p = n->parent;

	vr_bnode_t* s; // sibling
	vr_bnode_t* a; // new breakpoint

	// find sibling and breakpoint
	if (n == p->left)
	{
		s = p->right;
		a = vr_bnode_right(n);
		a->r2 = p->r2;
	}
	else
	{
		s = p->left;
		a = vr_bnode_left(n);
		a->r1 = p->r1;
	}

	// find where to put sibling
	vr_bnode_t* pp = p->parent;
	vr_bnode_t** x = p == pp->left ? &pp->left  : &pp->right;

	*x = s;
	s->parent = pp;

	free(n);
	free(p);

	return a;
}
