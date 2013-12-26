#include "swequipment.h"

void swequipment_init(swequipment_t* w)
{
	subwindow_init(&w->w, L"Equipment", 1024-SW_WIDTH*3, SW_HEIGHT);
}

void swequipment_exit(swequipment_t* w)
{
	subwindow_exit(&w->w);
}

void swequipment_draw(swequipment_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;
}

char swequipment_catch(swequipment_t* w, game_t* g, float x, float y, int t)
{
	(void) w;
	(void) g;
	(void) x;
	(void) y;
	(void) t;
	return 0;
}
