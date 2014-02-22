/*\
 *  Role playing, management and strategy game
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

#include "projectile.h"

#include <math.h>

#define M_PI 3.14159265358979323846

void projectile_init(projectile_t* p, world_t* w, kindOf_projectile_t* t, float x, float y, float damage, float tx, float ty)
{
	p->o.t = O_PROJECTILE;
	p->o.x = x;
	p->o.y = y;
	p->o.w = t->width;
	p->o.h = t->height;

	p->w = w;
	p->t = t;

	p->damage = damage;
	p->target_x = tx;
	p->target_y = ty;
	p->dir = D_SOUTH;
	p->step = 0;
}

void projectile_exit(projectile_t* p)
{
	(void) p;
}

char projectile_doRound(projectile_t* p, float duration)
{
	float dx = p->target_x - p->o.x;
	float dy = p->target_y - p->o.y;

	if (dx == 0 && dy == 0)
	{
		object_t* o = world_objectAt(p->w, p->target_x, p->target_y, NULL);
		if (o == NULL)
			return 0;

		if (p->t->event >= 0)
		{
			universe_t*  u = p->w->universe;
			kindOf_event_t* e = &u->events[p->t->event];
			evtList_push(&p->w->events, e, o->x, o->y - o->h/2);
		}

		if (o->t == O_CHARACTER)
		{
			character_t* c = (character_t*) o;
			character_attacked(c, p->damage);
		}
		else if (o->t == O_BUILDING)
		{
			building_t* b = (building_t*) o;
			building_attacked(b, p->damage);
		}
		return 0;
	}

	float dir = atan2f(dy, dx);

	p->dir = dir <-M_PI * 3/4 ? D_WEST :
	         dir <-M_PI * 1/4 ? D_NORTH :
	         dir < M_PI * 1/4 ? D_EAST :
	         dir < M_PI * 3/4 ? D_SOUTH :
	                            D_WEST;

	float distance = 150 * duration;
	float remDistance = sqrt(dx*dx + dy*dy);
	distance = fmin(distance, remDistance);

	p->o.x += distance * cos(dir);
	p->o.y += distance * sin(dir);

	p->step += 10 * duration;
	if (p->step >= 4)
		p->step = 0;

	return 1;
}
