#include "character.h"

#include <stdio.h>
#include <math.h>
#include <string.h>

#include "mine.h"

#define M_PI 3.14159265358979323846

void character_init(character_t* c)
{
	c->o.t = O_CHARACTER;
	c->o.x = 0;
	c->o.y = 0;
	c->o.w = 24;
	c->o.h = 32;
	c->go_x = 0;
	c->go_y = 0;
	c->go_o = NULL;
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
		if (c->go_o == NULL)
			return;

		if (c->go_o->t != O_MINE)
			return;

		mine_t* m = (mine_t*) c->go_o;
		printf("I just arrived at a %s\n", m->t->name);
		return;
	}

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

	float distance = 100 * duration;
	if (distance > remDistance)
		distance = remDistance;

	c->o.x += distance * cos(dir);
	c->o.y += distance * sin(dir);
}
