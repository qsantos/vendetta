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
	c->step = 5; // standing still

	inventory_init(&c->inventory, u);
	c->inBuilding = NULL;

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		c->sskills[i] = 1;
	c->mskills = CALLOC(skill_t, u->n_materials);
	for (int i = 0; i < u->n_materials; i++)
		c->mskills[i] = 1;
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

		if (m->t->harvest.n == 0 || m->t->harvest.c[0].is_item)
			return;

		int id = m->t->harvest.c[0].id;
		float work = duration * m->t->harvest.rate * c->mskills[id];
		components_apply(&m->t->harvest, &c->inventory, work);
		c->mskills[id] += work/100;
	}
	else if (o->t == O_BUILDING)
	{
		building_t* b = (building_t*) o;
		kindOf_building_t* t = b->t;

		if (b->build_progress != 1)
		{
			if (t->build_time == 0)
			{
				b->build_progress = 1;
			}
			else
			{
				float work = duration * c->sskills[S_BUILD] / t->build_time;
				b->build_progress += work;
				c->sskills[S_BUILD] += work/100;

				if (b->build_progress > 1)
					b->build_progress = 1;
			}
		}
		else if (b->item_current >= 0)
		{
			c->inBuilding = b;

			float ratio_max = 1 * duration;
			float ratio_rem = 1 - b->item_progress;
			if (ratio_max > ratio_rem)
				ratio_max = ratio_rem;

			int i = b->item_current;
			float ratio = components_ratio(&t->item_req[i], &c->inventory, ratio_max);
			components_apply(&t->item_req[i], &c->inventory, -ratio);
			b->item_progress += ratio;
			if (b->item_progress >= 1)
			{
				components_apply(&t->item_res[i], &c->inventory, +1);
				b->item_current = -1;
			}
		}
		else
		{
			c->inBuilding = b;

			if (t->make_res.n == 0 || t->make_res.c[0].is_item)
				return;

			int id = t->make_res.c[0].id;
			float work = duration * t->make_res.rate * c->mskills[id];
			float ratio = components_ratio(&t->make_req, &c->inventory, work);
			if (ratio != 0)
			{
				c->mskills[id] += ratio/100;

				components_apply(&t->make_req, &c->inventory, -ratio);
				components_apply(&t->make_res, &c->inventory, +ratio);
			}
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
		c->dir  = D_SOUTH;
		c->step = 5;
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

	if (c->step == 5)
		c->step = 0;
	c->step += 8 * duration;
	if (c->step >= 4)
		c->step = 0;
}
