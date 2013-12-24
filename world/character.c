#include "character.h"

#include <math.h>
#include <string.h>

#include "../util.h"
#include "mine.h"
#include "building.h"

#define M_PI 3.14159265358979323846

void character_init(character_t* c, universe_t* u)
{
	c->o.t = O_CHARACTER;
	c->o.x = 0;
	c->o.y = 0;
	c->o.w = 24;
	c->o.h = 32;

	c->go_x = c->o.x;
	c->go_y = c->o.y;
	c->go_o = NULL;
	c->dir  = D_SOUTH;

	inventory_init(&c->inventory, u);
	c->inBuilding = NULL;
}

void character_exit(character_t* c)
{
	inventory_exit(&c->inventory);
}

void character_workAt(character_t* c, object_t* o, float duration)
{
	if (o == NULL)
		return;

	if (o->t == O_MINE)
	{
		mine_t* m = (mine_t*) o;
		components_apply(&m->t->harvest, &c->inventory, 1 * duration);
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;
		kindOf_building_t* t = b->t;

		if (b->buildProgress == 1)
		{
			c->inBuilding = b;

			float ratio = components_ratio(&t->make_req, &c->inventory, 1 * duration);
			if (ratio != 0)
			{
				components_apply(&t->make_req, &c->inventory, -ratio);
				components_apply(&t->make_res, &c->inventory, +ratio);
			}
		}
		else if (t->build_time == 0)
		{
			b->buildProgress = 1;
		}
		else
		{
			b->buildProgress += duration / t->build_time;
			if (b->buildProgress > 1)
				b->buildProgress = 1;
		}
	}
}

void character_doRound(character_t* c, float duration)
{
	float dx;
	float dy;
	if (c->go_o != NULL)
	{
		dx = c->go_o->x;
		dy = c->go_o->y;
	}
	else
	{
		dx = c->go_x;
		dy = c->go_y;
	}
	dx -= c->o.x;
	dy -= c->o.y;

	float remDistance = sqrt(dx*dx + dy*dy);
	if (remDistance == 0)
	{
		c->dir = D_SOUTH;
		character_workAt(c, c->go_o, duration);
		return;
	}
	c->inBuilding = NULL;

	float dir;
	if (dx > 0)
	{
		dir = atan(dy / dx);
		if (dy < 0)
			dir += 2 * M_PI;
	}
	else if (dx < 0)
	{
		dir = atan(dy / dx) + M_PI;
	}
	else // dx == 0
	{
		dir = M_PI / 2;
		if (dy < 0)
			dir += M_PI;
	}

	c->dir = dir < M_PI * 1/4 ? D_EAST :
	         dir < M_PI * 3/4 ? D_SOUTH :
		 dir < M_PI * 5/4 ? D_WEST :
		 dir < M_PI * 7/4 ? D_NORTH :
		                     D_EAST;

	float distance = 100 * duration;
	if (distance > remDistance)
		distance = remDistance;

	c->o.x += distance * cos(dir);
	c->o.y += distance * sin(dir);
}
