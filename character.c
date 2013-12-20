#include "character.h"

#include <math.h>

#define M_PI 3.14159265358979323846

void character_doRound(character_t* c, float duration)
{
	if (c->go_x == 0 && c->go_y == 0)
		return;

	float dir;
	if (c->go_x > 0)
	{
		dir = atan(c->go_y / c->go_x);
		if (c->go_y < 0)
			dir += 2 * M_PI;
	}
	else if (c->go_x < 0)
	{
		dir = atan(c->go_y / c->go_x) + M_PI;
	}
	else // c->go_x == 0
	{
		dir = M_PI / 2;
		if (c->go_y < 0)
			dir += M_PI;
	}

	float maxDistance = 100 * duration;

	c->x += maxDistance * cos(dir);
	c->y += maxDistance * sin(dir);
}
