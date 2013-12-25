#include "object.h"

#include <math.h>

#include "character.h"
#include "mine.h"

int object_isAt(object_t* o, float x, float y)
{
	return fabs(x - o->x) <= o->w/2 && o->y - o->h <= y && y <= o->y;
}

int object_rect(object_t* o, float x, float y, float w, float h)
{
	x -= o->x;
	y -= o->y;
	float hori = fabs(x) <= (w + o->w)/2;
	float vert = -o->h <= y && y < h;
	return hori && vert;
}
