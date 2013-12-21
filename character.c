#include "character.h"

#include <math.h>
#include <string.h>

#define M_PI 3.14159265358979323846

void character_init(character_t* c)
{
	memset(c, 0, sizeof(character_t));
	c->o.t = O_CHARACTER;
}

void character_doRound(character_t* c, float duration)
{
	float dx = c->go_x - c->o.x;
	float dy = c->go_y - c->o.y;
	if (dx == 0 && dy == 0)
		return;

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

	float remDistance = sqrt(dx*dx + dy*dy);
	float distance = 100 * duration;
	if (distance > remDistance)
		distance = remDistance;

	c->o.x += distance * cos(dir);
	c->o.y += distance * sin(dir);
}
