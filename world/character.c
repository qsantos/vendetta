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

	c->universe = u;
	inventory_init(&c->inventory, u);
	c->inBuilding = NULL;

	for (int i = 0; i < N_SPECIAL_SKILLS; i++)
		c->sskills[i] = 1;

	c->mskills = CALLOC(skill_t, u->n_materials);
	for (int i = 0; i < u->n_materials; i++)
		c->mskills[i] = 1;

	c->iskills = CALLOC(skill_t, u->n_iskills);
	for (int i = 0; i < u->n_iskills; i++)
		c->iskills[i] = 1;

	for (int i = 0; i < N_STATUSES; i++)
		c->statuses[i] = 20;

	c->equipment = CALLOC(int, u->n_slots);
	for (int i = 0; i < u->n_slots; i++)
		c->equipment[i] = -1;

	// just for convenience
	for (int i = 0; i < u->n_materials; i++)
		c->inventory.materials[i] = 1000;
}

void character_exit(character_t* c)
{
	free(c->equipment);
	free(c->iskills);
	free(c->mskills);
	inventory_exit(&c->inventory);
}

float character_vitality(character_t* c)
{
	float ret = 1;
	ret *= sqrt(c->statuses[ST_HEALTH] / 20.);
	ret *= sqrt(c->statuses[ST_STAMINA] / 20.);
	ret *= sqrt(c->statuses[ST_MORAL] / 20.);
	return max(ret, 0.3);
}

void character_weary(character_t* c, float f)
{
	c->statuses[ST_STAMINA] = max(c->statuses[ST_STAMINA] - f,   0);
	c->statuses[ST_MORAL]   = max(c->statuses[ST_MORAL]   - f/3, 0);
}

void character_workAt(character_t* c, object_t* o, float duration)
{
	if (o == NULL)
	{
		character_weary(c, 0.01 * duration);
		return;
	}

	if (o->t == O_MINE)
	{
		mine_t* m = (mine_t*) o;

		if (m->t->harvest.n == 0 || m->t->harvest.c[0].is_item)
			return;

		int id = m->t->harvest.c[0].id;

		float work = duration * m->t->harvest.rate;
		work *= c->mskills[id];
		work *= character_vitality(c);

		components_apply(&m->t->harvest, &c->inventory, work);

		c->mskills[id] += duration/100;
		character_weary(c, 0.1 * duration);
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
				float work = 1 * duration;
				work *= c->sskills[SK_BUILD];
				work *= character_vitality(c);

				float rem = (1 - b->build_progress)*b->t->build_time;
				if (work > rem)
					work = rem;

				b->build_progress += work / b->t->build_time;

				c->sskills[SK_BUILD] += duration/100;
				character_weary(c, 0.3 * duration);
			}
		}
		else if (b->item_current >= 0)
		{
			c->inBuilding = b;
			int i = b->item_current;

			if (t->item_res[i].n == 0 || !t->item_res[i].c[0].is_item)
				return;

			universe_t* u = c->universe;
			int id = u->items[t->item_res[i].c[0].id].skill;

			float work = 1 * duration;
			work *= c->iskills[id];
			work *= character_vitality(c);

			float rem  = (1 - b->item_progress)/t->item_req[i].rate;
			if (work > rem)
				work = rem;

			float ratio = components_ratio(&t->item_req[i], &c->inventory, work * t->item_req[i].rate);
			if (ratio != 0)
			{
				components_apply(&t->item_req[i], &c->inventory, -ratio);
				b->item_progress += ratio;
				if (b->item_progress >= 1)
				{
					components_apply(&t->item_res[i], &c->inventory, +1);
					b->item_current = -1;
				}

				c->iskills[id] += duration/100;
				character_weary(c, 0.1 * duration);
			}
		}
		else
		{
			c->inBuilding = b;

			if (t->make_res.n == 0 || t->make_res.c[0].is_item)
				return;

			int id = t->make_res.c[0].id;

			float work = 1 * duration;
			work *= c->mskills[id];
			work *= character_vitality(c);

			float ratio = components_ratio(&t->make_req, &c->inventory, work * t->make_res.rate);
			if (ratio != 0)
			{
				components_apply(&t->make_req, &c->inventory, -ratio);
				components_apply(&t->make_res, &c->inventory, +ratio);

				c->mskills[id] += duration/100;
				character_weary(c, 0.1 * duration);
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
	distance *= character_vitality(c);
	if (distance > remDistance)
		distance = remDistance;

	c->o.x += distance * cos(dir);
	c->o.y += distance * sin(dir);

	if (c->step == 5)
		c->step = 0;
	c->step += 0.1 * distance;
	if (c->step >= 4)
		c->step = 0;

	character_weary(c, 0.02*duration);
}
